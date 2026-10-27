//! Reproducible phase-1 content inventory. No ROM data leaves the process.
use sml2_core::collision::classify;
use sml2_dx::{extract, Presentation};
use sml2_extract::vanilla::{decompress_level, identify, read_packed_stream};
use std::{env, fs, path::PathBuf};

fn word(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn offset(bank: u8, address: usize) -> usize {
    assert!((0x4000..0x8000).contains(&address));
    usize::from(bank) * 0x4000 + address - 0x4000
}

fn header(rom: &[u8], id: usize) -> &[u8] {
    let at = word(rom, 0x55cb + id * 2);
    &rom[at..at + 20]
}

fn layout(rom: &[u8], header: &[u8]) -> Vec<u8> {
    let bank = header[13];
    let pointer = offset(bank, 0x42f0 + usize::from(header[18] & 15) * 2);
    let start = offset(bank, word(rom, pointer));
    let end = (usize::from(bank) + 1) * 0x4000;
    decompress_level(&rom[start..end]).unwrap()
}

fn map<'a>(rom: &'a [u8], header: &[u8], table: usize) -> &'a [u8] {
    let bank = header[13];
    let at = offset(bank, word(rom, usize::from(bank) * 0x4000 + table))
        + usize::from(header[18] & 15) * 48;
    &rom[at..at + 48]
}

fn roms() -> (Vec<u8>, Vec<u8>) {
    let dx = fs::read(env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    let base_path = env::var_os("SML2_BASE_ROM")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../sml2decomp/baserom.gb")
        });
    let base = fs::read(base_path).unwrap();
    identify(&base).unwrap();
    sml2_dx::identify(&dx).unwrap();
    (base, dx)
}

#[test]
#[ignore = "requires SML2_DX_ROM and the audited base ROM (SML2_BASE_ROM overrides sibling path)"]
fn audited_dx_content_difference_inventory() {
    let (base, dx) = roms();
    let bundle = extract(&dx).unwrap();
    let presentation = Presentation::new(&bundle).unwrap();

    // Per level: changed metatile bytes, decompressed cells, collision classes.
    let expected = [
        (0, 0, 0),
        (8, 3473, 45),
        (0, 0, 0),
        (21, 39, 0),
        (10, 301, 0),
        (11, 128, 0),
        (4, 790, 768),
        (20, 0, 0),
        (10, 0, 0),
        (22, 0, 0),
        (0, 0, 0),
        (79, 773, 0),
        (0, 256, 0),
        (118, 1739, 445),
        (101, 184, 5),
        (0, 0, 0),
        (0, 0, 0),
        (101, 372, 1),
        (8, 0, 0),
        (8, 0, 0),
        (24, 4402, 0),
        (20, 470, 0),
        (0, 0, 0),
        (2, 0, 0),
        (0, 0, 0),
        (0, 0, 0),
        (0, 0, 0),
        (40, 347, 1),
        (8, 0, 0),
        (8, 1147, 0),
        (11, 0, 0),
        (0, 0, 0),
    ];
    let mut changed_headers = Vec::new();
    let mut changed_spawns = Vec::new();
    for (id, expected) in expected.into_iter().enumerate() {
        let bh = header(&base, id);
        let dh = header(&dx, id);
        for field in 0..20 {
            if bh[field] != dh[field] {
                changed_headers.push((id, field, bh[field], dh[field]));
            }
        }
        let bm = offset(8, word(bh, 11));
        let dm = offset(8, word(dh, 11));
        let metatile_bytes = base[bm..bm + 512]
            .iter()
            .zip(&dx[dm..dm + 512])
            .filter(|(b, d)| b != d)
            .count();
        let bl = layout(&base, bh);
        let dl = layout(&dx, dh);
        assert_eq!(
            presentation.levels[id].layout, dl,
            "DX decoder level {id:02X}"
        );
        let cells = bl.iter().zip(&dl).filter(|(b, d)| b != d).count();
        let classes = bl
            .iter()
            .zip(&dl)
            .filter(|(b, d)| classify(**b) != classify(**d))
            .count();
        assert_eq!((metatile_bytes, cells, classes), expected, "level {id:02X}");
        // All class changes in this revision cross the solid/non-solid boundary.
        assert_eq!(
            bl.iter()
                .zip(&dl)
                .filter(|(b, d)| (**b < 0x38) != (**d < 0x38))
                .count(),
            classes
        );
        assert_eq!(map(&base, bh, 0), map(&dx, dh, 0), "edge map {id:02X}");
        assert_eq!(map(&base, bh, 2), map(&dx, dh, 2), "scene map {id:02X}");
        let bs = read_packed_stream(&base, offset(3, word(&base, 0xe037 + id * 2))).unwrap();
        let ds = read_packed_stream(&dx, offset(3, word(&dx, 0xe037 + id * 2))).unwrap();
        if bs != ds {
            changed_spawns.push(id);
            assert_eq!(bs.len(), ds.len());
            let differences: Vec<_> = bs
                .iter()
                .zip(&ds)
                .enumerate()
                .filter(|(_, (b, d))| b != d)
                .map(|(i, (&b, &d))| (i, b, d))
                .collect();
            assert_eq!(differences, [(92, 0x31, 0x32)]);
        }
        println!("{id:02X}: metatile_bytes={metatile_bytes} layout_cells={cells} collision_cells={classes}");
    }
    assert_eq!(changed_headers, [(0x1b, 12, 0x42, 0x78)]);
    assert_eq!(changed_spawns, [0x0b]);
    for (name, start, end) in [
        ("collision consumers", 0x1c82, 0x1f51),
        ("level properties", 0x1f51, 0x1fb1),
        ("arena metatile pointers", 0x26b5, 0x26c3),
        (
            "secret entries including overlapping final row",
            0x377b,
            0x387b,
        ),
        ("spawn pointers", 0xe037, 0xe077),
        ("scene scripts", 0x14125, 0x141e5),
    ] {
        assert_eq!(base[start..end], dx[start..end], "{name}");
    }
    let handler_changes: Vec<_> = (0x58000..0x5c000).filter(|&i| base[i] != dx[i]).collect();
    assert_eq!(handler_changes, [0x5872b, 0x5876a, 0x5b693]);
    for (at, old, new) in [(0x5872b, 0x80, 0), (0x5876a, 0x80, 0), (0x5b693, 0x10, 8)] {
        assert_eq!((base[at], dx[at]), (old, new));
    }
}

#[test]
#[ignore = "requires SML2_DX_ROM and the audited base ROM (SML2_BASE_ROM overrides sibling path)"]
fn audited_dx_character_table_inventory() {
    let (base, dx) = roms();
    let differences = |left: &[u8], right: &[u8]| -> Vec<(usize, u8, u8)> {
        assert_eq!(left.len(), right.len());
        left.iter()
            .zip(right)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (&a, &b))| (i, a, b))
            .collect()
    };
    // Mario's relocated curves and both characters' shared swim/moon curves.
    for (source, target, len) in [
        (0x19ff, 0x4000, 256),
        (0x18af, 0x4200, 76),
        (0x18fb, 0x424c, 76),
        (0x18fb, 0x4530, 76),
        (0x1947, 0x4298, 76),
        (0x1947, 0x457c, 76),
    ] {
        assert_eq!(base[source..source + len], dx[target..target + len]);
    }
    assert_eq!(
        differences(&base[0x19ff..0x1aff], &dx[0x42e4..0x43e4]),
        [(0xf7, 1, 2), (0xf8, 2, 3)]
    );
    assert_eq!(
        differences(&base[0x18af..0x18fb], &dx[0x44e4..0x4530]),
        (59..=72).map(|i| (i, 0xfd, 0xfc)).collect::<Vec<_>>()
    );
    assert_eq!(
        differences(&base[0x1aff..0x1bff], &dx[0x1aff..0x1bff]),
        [(0, 0xff, 0)]
    );
    let config = offset(0x24, 0x7d00);
    assert_eq!(dx[config..config + 64], [0xff; 64]);
    assert_eq!(dx[config + 64..config + 96], [1; 32]);
    for (start, end) in [
        (0x0f71, 0x13fe),
        (0x13fe, 0x1501),
        (0x150c, 0x156b),
        (0x1590, 0x16e9),
        (0x1701, 0x1842),
        (0x1854, 0x18af),
        (0x1bff, 0x1d72),
        (0x31ef, 0x324f),
    ] {
        assert_eq!(
            base[start..end],
            dx[start..end],
            "unchanged path {start:04X}..{end:04X}"
        );
    }
}
