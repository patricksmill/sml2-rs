use super::*;

fn blobs(tag: u8) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        vec![tag; VRAM_LEN],
        vec![tag ^ 1; TILEMAP_LEN],
        vec![tag ^ 2; VRAM_LEN],
        vec![tag ^ 3; TILEMAP_LEN],
        vec![tag ^ 4; DEMO_TAPES_LEN],
    )
}

#[test]
fn accessors_return_the_bundled_blobs() {
    let (title_vram, title_map, select_vram, select_map, tapes) = blobs(0x11);
    let assets = FrontEndAssets::new(
        title_vram.clone(),
        title_map.clone(),
        select_vram.clone(),
        select_map.clone(),
        tapes.clone(),
    )
    .unwrap();
    assert_eq!(assets.title_vram(), title_vram.as_slice());
    assert_eq!(assets.title_tilemap(), title_map.as_slice());
    assert_eq!(assets.file_select_vram(), select_vram.as_slice());
    assert_eq!(assets.file_select_tilemap(), select_map.as_slice());
    assert_eq!(assets.demo_tapes(), tapes.as_slice());
}

#[test]
fn demo_pages_run_from_page_three_down_to_page_zero() {
    let (title_vram, title_map, select_vram, select_map, mut tapes) = blobs(0);
    for (page, byte) in tapes.chunks_mut(DEMO_TAPE_LEN).enumerate() {
        byte.fill(page as u8);
    }
    let assets = FrontEndAssets::new(title_vram, title_map, select_vram, select_map, tapes).unwrap();
    // $7C00 (page 3) is demo 3; $7F00 (page 0) is demo 0.
    assert_eq!(assets.demo_tape(3)[0], 0);
    assert_eq!(assets.demo_tape(2)[0], 1);
    assert_eq!(assets.demo_tape(1)[0], 2);
    assert_eq!(assets.demo_tape(0)[0], 3);
    assert_eq!(assets.demo_tape(4), assets.demo_tape(0));
}

#[test]
fn every_blob_must_match_the_source_length() {
    let (title_vram, title_map, select_vram, select_map, tapes) = blobs(0x20);
    for (index, name) in [
        "title_vram",
        "title_tilemap",
        "file_select_vram",
        "file_select_tilemap",
        "demo_tapes",
    ]
    .iter()
    .enumerate()
    {
        let mut parts = [
            title_vram.clone(),
            title_map.clone(),
            select_vram.clone(),
            select_map.clone(),
            tapes.clone(),
        ];
        parts[index].pop();
        let result = FrontEndAssets::new(
            parts[0].clone(),
            parts[1].clone(),
            parts[2].clone(),
            parts[3].clone(),
            parts[4].clone(),
        );
        assert!(
            result.unwrap_err().starts_with(name),
            "the {name} length error must name the blob"
        );
        parts[index].push(0);
        assert!(FrontEndAssets::new(
            parts[0].clone(),
            parts[1].clone(),
            parts[2].clone(),
            parts[3].clone(),
            parts[4].clone(),
        )
        .is_ok());
    }
}
