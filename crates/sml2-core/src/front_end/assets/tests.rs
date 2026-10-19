use super::*;

fn blobs(tag: u8) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        vec![tag; VRAM_LEN],
        vec![tag ^ 1; TILEMAP_LEN],
        vec![tag ^ 2; VRAM_LEN],
        vec![tag ^ 3; TILEMAP_LEN],
    )
}

#[test]
fn accessors_return_the_bundled_blobs() {
    let (title_vram, title_map, select_vram, select_map) = blobs(0x11);
    let assets = FrontEndAssets::new(
        title_vram.clone(),
        title_map.clone(),
        select_vram.clone(),
        select_map.clone(),
    )
    .unwrap();
    assert_eq!(assets.title_vram(), title_vram.as_slice());
    assert_eq!(assets.title_tilemap(), title_map.as_slice());
    assert_eq!(assets.file_select_vram(), select_vram.as_slice());
    assert_eq!(assets.file_select_tilemap(), select_map.as_slice());
}

#[test]
fn every_blob_must_match_the_source_length() {
    let (title_vram, title_map, select_vram, select_map) = blobs(0x20);
    for (index, name) in ["title_vram", "title_tilemap", "file_select_vram", "file_select_tilemap"]
        .iter()
        .enumerate()
    {
        let mut parts = [
            title_vram.clone(),
            title_map.clone(),
            select_vram.clone(),
            select_map.clone(),
        ];
        parts[index].pop();
        let result = FrontEndAssets::new(parts[0].clone(), parts[1].clone(), parts[2].clone(), parts[3].clone());
        assert!(
            result.unwrap_err().starts_with(name),
            "the {name} length error must name the blob"
        );
        parts[index].push(0);
        assert!(
            FrontEndAssets::new(parts[0].clone(), parts[1].clone(), parts[2].clone(), parts[3].clone())
                .is_ok()
        );
    }
}
