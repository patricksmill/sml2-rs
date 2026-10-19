use super::*;
use crate::ending_scene::{EndingAssets, EndingScene};

#[test]
fn test_chunk_len_aligned_and_unaligned() {
    let t_zero = VramTransfer::new(0, 0, 0, 0);
    assert_eq!(t_zero.chunk_len(), 0);

    let t_64 = VramTransfer::new(0, 0, 0, 64);
    assert_eq!(t_64.chunk_len(), 64);

    let t_128 = VramTransfer::new(0, 0, 0, 128);
    assert_eq!(t_128.chunk_len(), 64);

    let t_2048 = VramTransfer::new(0, 0, 0, 2048);
    assert_eq!(t_2048.chunk_len(), 64);

    // Unaligned: copies remainder first down to next 64-byte boundary
    let t_21 = VramTransfer::new(0, 0, 0, 21);
    assert_eq!(t_21.chunk_len(), 21);

    let t_50 = VramTransfer::new(0, 0, 0, 50);
    assert_eq!(t_50.chunk_len(), 50);

    let t_70 = VramTransfer::new(0, 0, 0, 70);
    assert_eq!(t_70.chunk_len(), 6); // 70 & 0x3F = 6

    let t_65 = VramTransfer::new(0, 0, 0, 65);
    assert_eq!(t_65.chunk_len(), 1); // 65 & 0x3F = 1
}

#[test]
fn test_step_buffer_64_byte_aligned() {
    let mut transfer = VramTransfer::new(1, 0x10, 0x20, 128);
    let source: Vec<u8> = (0..200).collect();
    let mut dest = vec![0u8; 200];

    // Frame 1
    let r1 = transfer
        .step_buffer(&source, 0x10, &mut dest, 0x20)
        .unwrap();
    assert_eq!(r1.bytes_transferred, 64);
    assert_eq!(r1.remaining_bytes, 64);
    assert!(!r1.completed);
    assert!(transfer.active);
    assert_eq!(transfer.source_addr, 0x10 + 64);
    assert_eq!(transfer.dest_addr, 0x20 + 64);
    assert_eq!(&dest[0x20..0x20 + 64], &source[0x10..0x10 + 64]);

    // Frame 2
    let r2 = transfer
        .step_buffer(&source, 0x10 + 64, &mut dest, 0x20 + 64)
        .unwrap();
    assert_eq!(r2.bytes_transferred, 64);
    assert_eq!(r2.remaining_bytes, 0);
    assert!(r2.completed);
    assert!(!transfer.active);
    assert_eq!(&dest[0x20..0x20 + 128], &source[0x10..0x10 + 128]);

    // Frame 3 (already done)
    let r3 = transfer
        .step_buffer(&source, 0x10 + 128, &mut dest, 0x20 + 128)
        .unwrap();
    assert_eq!(r3.bytes_transferred, 0);
    assert!(r3.completed);
}

#[test]
fn test_step_buffer_unaligned() {
    let mut transfer = VramTransfer::new(1, 0, 0, 70);
    let source: Vec<u8> = (0..100).collect();
    let mut dest = vec![0u8; 100];

    // Frame 1: copies 6 bytes (down to 64)
    let r1 = transfer.step_buffer(&source, 0, &mut dest, 0).unwrap();
    assert_eq!(r1.bytes_transferred, 6);
    assert_eq!(r1.remaining_bytes, 64);
    assert!(transfer.active);

    // Frame 2: copies remaining 64 bytes
    let r2 = transfer.step_buffer(&source, 6, &mut dest, 6).unwrap();
    assert_eq!(r2.bytes_transferred, 64);
    assert_eq!(r2.remaining_bytes, 0);
    assert!(r2.completed);
    assert!(!transfer.active);
    assert_eq!(&dest[0..70], &source[0..70]);
}

#[test]
fn test_ending_request_1_full_run() {
    let mut transfer = VramTransfer::ending_request_1();
    assert_eq!(transfer.source_bank, 0x1A);
    assert_eq!(transfer.source_addr, 0x4656);
    assert_eq!(transfer.dest_addr, 0x8000);
    assert_eq!(transfer.remaining_bytes, 2048);
    assert!(transfer.active);

    let rom_start = VramTransfer::rom_offset(0x1A, 0x4656);
    assert_eq!(rom_start, 26 * 0x4000 + (0x4656 - 0x4000));
    assert_eq!(rom_start, 0x68656);

    let source: Vec<u8> = (0..0x80000).map(|i| (i & 0xFF) as u8).collect();
    let mut dest = vec![0u8; 0x10000];

    for frame in 1..=32 {
        let dest_offset = transfer.dest_addr as usize;
        let res = transfer.step_rom(&source, &mut dest, dest_offset).unwrap();
        assert_eq!(res.bytes_transferred, 64);
        assert_eq!(res.remaining_bytes, 2048 - (frame * 64) as u16);
        if frame < 32 {
            assert!(!res.completed);
            assert!(transfer.active);
        } else {
            assert!(res.completed);
            assert!(!transfer.active);
        }
    }

    assert_eq!(transfer.source_addr, 0x4656 + 2048);
    assert_eq!(transfer.dest_addr, 0x8000 + 2048);
    assert_eq!(
        &dest[0x8000..0x8000 + 2048],
        &source[0x68656..0x68656 + 2048]
    );
}

#[test]
fn test_ending_request_2_full_run() {
    let mut transfer = VramTransfer::ending_request_2();
    assert_eq!(transfer.source_bank, 0x1A);
    assert_eq!(transfer.source_addr, 0x5096);
    assert_eq!(transfer.dest_addr, 0x9800);
    assert_eq!(transfer.remaining_bytes, 1024);

    let rom_start = VramTransfer::rom_offset(0x1A, 0x5096);
    assert_eq!(rom_start, 0x69096);

    let source: Vec<u8> = (0..0x80000).map(|i| (i ^ 0x55) as u8).collect();
    let mut dest = vec![0u8; 0x10000];

    for frame in 1..=16 {
        let dest_offset = transfer.dest_addr as usize;
        let res = transfer.step_rom(&source, &mut dest, dest_offset).unwrap();
        assert_eq!(res.bytes_transferred, 64);
        assert_eq!(res.remaining_bytes, 1024 - (frame * 64) as u16);
        if frame < 16 {
            assert!(!res.completed);
            assert!(transfer.active);
        } else {
            assert!(res.completed);
            assert!(!transfer.active);
        }
    }

    assert_eq!(transfer.source_addr, 0x5096 + 1024);
    assert_eq!(transfer.dest_addr, 0x9800 + 1024);
    assert_eq!(
        &dest[0x9800..0x9800 + 1024],
        &source[0x69096..0x69096 + 1024]
    );
}

#[test]
fn test_step_scene_tiles_and_tilemap() {
    let mut scene = EndingScene::default();
    let initial_assets = EndingAssets::new(vec![0xAA; 6144], vec![0xBB; 1024]).unwrap();
    let mut ending_state = 0;
    let mut fade = 0;
    scene.load(initial_assets, &mut ending_state, &mut fade);

    let rom: Vec<u8> = (0..0x80000).map(|i| (i as u8).wrapping_mul(3)).collect();

    // Step request 1 (copies into tiles $8000)
    let mut req1 = VramTransfer::ending_request_1();
    while req1.active {
        req1.step_scene(&rom, &mut scene).unwrap();
    }
    assert_eq!(
        &scene.assets().unwrap().tiles()[0..2048],
        &rom[0x68656..0x68656 + 2048]
    );
    // Bytes after 2048 in tiles remain untouched
    assert_eq!(
        &scene.assets().unwrap().tiles()[2048..2048 + 32],
        &[0xAA; 32]
    );

    // Step request 2 (copies into tilemap $9800)
    let mut req2 = VramTransfer::ending_request_2();
    while req2.active {
        req2.step_scene(&rom, &mut scene).unwrap();
    }
    assert_eq!(
        &scene.assets().unwrap().tilemap()[0..1024],
        &rom[0x69096..0x69096 + 1024]
    );
}

#[test]
fn test_transfer_errors() {
    let mut t = VramTransfer::new(0, 100, 0x8000, 64);
    let mut dest = vec![0u8; 100];
    let short_src = vec![0u8; 50];

    assert_eq!(
        t.step_buffer(&short_src, 100, &mut dest, 0),
        Err(TransferError::SourceOutOfBounds {
            required: 164,
            available: 50,
        })
    );

    let src = vec![0u8; 200];
    let mut short_dest = vec![0u8; 30];
    assert_eq!(
        t.step_buffer(&src, 0, &mut short_dest, 0),
        Err(TransferError::DestinationOutOfBounds {
            dest_addr: 0x8000,
            length: 64,
        })
    );

    let mut empty_scene = EndingScene::default();
    assert_eq!(
        t.step_scene(&src, &mut empty_scene),
        Err(TransferError::SceneAssetsMissing)
    );
}
