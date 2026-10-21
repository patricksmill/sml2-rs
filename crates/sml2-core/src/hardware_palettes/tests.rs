use super::*;

// Actual supported cartridge $0134..=$014B, not a palette-selector fixture.
const SML2_HEADER: [u8; 0x18] = [
    0x4D, 0x41, 0x52, 0x49, 0x4F, 0x4C, 0x41, 0x4E, 0x44, 0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 2,
    1, 1,
];

fn sml2_expected() -> HardwarePalettes {
    HardwarePalettes {
        bg: [0x67FF, 0x77AC, 0x1A13, 0x2D6B],
        obj0: [0x7FFF, 0x01DF, 0x0112, 0],
        obj1: [0x7FFF, 0x7E8C, 0x7C00, 0],
    }
}

fn default_expected() -> HardwarePalettes {
    HardwarePalettes {
        bg: [0x7FFF, 0x1BEF, 0x6180, 0],
        obj0: [0x7FFF, 0x421F, 0x1CF2, 0],
        obj1: [0x7FFF, 0x421F, 0x1CF2, 0],
    }
}

#[test]
fn supported_sml2_header_selects_three_exact_boot_palettes() {
    assert_eq!(
        SML2_HEADER[..16]
            .iter()
            .fold(0u8, |sum, &b| sum.wrapping_add(b)),
        0xC9
    );
    assert_eq!(SML2_HEADER[0x17], 1);
    assert_eq!(gbc_auto(&SML2_HEADER), sml2_expected());

    let root = std::env::var_os("SML2_DECOMP_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sml2decomp")
        });
    if let Ok(rom) = std::fs::read(root.join("baserom.gb")) {
        assert_eq!(&rom[0x134..0x14C], &SML2_HEADER);
        assert_eq!(
            gbc_auto(rom[0x134..0x14C].try_into().unwrap()),
            sml2_expected()
        );
    }
}

#[test]
fn nintendo_old_and_new_licensee_gates_and_unknown_title_default() {
    let mut header = SML2_HEADER;
    header[0x17] = 0x33;
    header[0x10..0x12].copy_from_slice(b"01");
    assert_eq!(gbc_auto(&header), sml2_expected());
    for (old, new) in [(0, *b"01"), (0x33, *b"11"), (0x33, *b"00")] {
        header[0x17] = old;
        header[0x10..0x12].copy_from_slice(&new);
        assert_eq!(gbc_auto(&header), default_expected());
    }
    header = [0; 0x18];
    header[0] = 0x02; // No checksum-table entry.
    header[0x17] = 1;
    assert_eq!(gbc_auto(&header), default_expected());
}

#[test]
fn checksum_covers_all_sixteen_bytes_and_wraps_at_eight_bits() {
    let mut header = [0; 0x18];
    header[0] = 0xFF;
    header[15] = 0xCA; // $FF + $CA wraps to SML2's $C9.
    header[0x17] = 1;
    assert_eq!(gbc_auto(&header), sml2_expected());
}

fn collision_header(sum: u8, fourth: u8) -> [u8; 0x18] {
    let mut header = [0; 0x18];
    header[0] = sum.wrapping_sub(fourth);
    header[3] = fourth;
    header[0x17] = 1;
    header
}

#[test]
fn collision_fourth_letter_selects_later_row_or_default() {
    // $B3: B -> first collision row; U -> second; R -> third.
    assert_eq!(
        gbc_auto(&collision_header(0xB3, b'B')),
        HardwarePalettes {
            bg: [0x7E74, 0x03FF, 0x0180, 0],
            obj0: [0x299F, 0x001A, 0x000C, 0],
            obj1: [0x7C00, 0x7FFF, 0x3FFF, 0x7E00], // Unaligned source window.
        }
    );
    assert_eq!(
        gbc_auto(&collision_header(0xB3, b'U')),
        HardwarePalettes {
            bg: [0x7FFF, 0x42B5, 0x3DC8, 0],
            obj0: [0x7FFF, 0x01DF, 0x0112, 0],
            obj1: [0x7FFF, 0x01DF, 0x0112, 0],
        }
    );
    assert_eq!(
        gbc_auto(&collision_header(0xB3, b'R')),
        HardwarePalettes {
            bg: [0x7FFF, 0x03EA, 0x011F, 0],
            obj0: [0x7FFF, 0x03EA, 0x011F, 0],
            obj1: [0x7FFF, 0x7EEB, 0x001F, 0x7C00],
        }
    );
    assert_eq!(gbc_auto(&collision_header(0xB3, b'!')), default_expected());
    // Super Mario Land uses a palette beginning one color before palette 4.
    assert_eq!(
        gbc_auto(&collision_header(0x46, b'E')).obj0,
        [0, 0x7FFF, 0x421F, 0x1CF2]
    );
}

fn fingerprint(bytes: impl IntoIterator<Item = u8>) -> u32 {
    bytes.into_iter().fold(0x811C9DC5u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x01000193)
    })
}

#[test]
fn hardware_tables_match_source_bytes() {
    // FNV-1a computed directly from pinned source tables, before Rust emission.
    assert_eq!(fingerprint(TITLE_CHECKSUMS), 0xE1230678);
    assert_eq!(fingerprint(PALETTE_PER_CHECKSUM), 0x324AFBFC);
    assert_eq!(
        fingerprint(GBC_COLORS.into_iter().flat_map(u16::to_le_bytes)),
        0x6ABCDE0D
    );
    assert_eq!(
        fingerprint(GBC_COMBINATIONS.into_iter().flatten().map(|x| x as u8)),
        0x947BF888
    );
    assert_eq!(
        fingerprint(SGB_PRESETS.into_iter().flatten().flat_map(u16::to_le_bytes)),
        0xCA41800D
    );
}

#[test]
fn sgb_system_rows_and_bounds() {
    assert_eq!(
        sgb_preset(0),
        Some(HardwarePalettes {
            bg: [0x67BF, 0x265B, 0x10B5, 0x2866],
            obj0: [0x67BF, 0x265B, 0x10B5, 0x2866],
            obj1: [0x67BF, 0x265B, 0x10B5, 0x2866],
        })
    );
    assert_eq!(sgb_preset(31).unwrap().bg, [0x67FF, 0x2F17, 0x2230, 0x1548]);
    assert_eq!(sgb_preset(32), None);
    assert_eq!(sgb_preset(usize::MAX), None);
}

#[test]
fn rgb555_uses_uncorrected_bit_replication() {
    assert_eq!(rgb555(0), [0, 0, 0]);
    assert_eq!(rgb555(0xFFFF), [255, 255, 255]);
    assert_eq!(rgb555(0x001F), [255, 0, 0]);
    assert_eq!(rgb555(0x03E0), [0, 255, 0]);
    assert_eq!(rgb555(0x7C00), [0, 0, 255]);
    assert_eq!(rgb555(0x77AC), [99, 239, 239]);
}
