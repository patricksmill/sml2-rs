//! Supported-ROM call ordering and special pose-path reachability.
use std::{env, fs};

#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_regional_pose_call_sites_and_scene_pump_gates() {
    let rom = fs::read(env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    sml2_dx::identify(&rom).unwrap();
    // Normal pose call precedes the object engine that stages the next refs.
    assert_eq!(&rom[0x0618..0x061b], &[0xcd, 0x0a, 0x2d]);
    assert_eq!(&rom[0x0636..0x0639], &[0xcd, 0x52, 0x06]);
    let at = 2 * 0x4000 + 0x4100 - 0x4000;
    assert_eq!(
        &rom[at..at + 0x20],
        &[
            0xfa, 0xc2, 0xff, 0xc6, 8, 0xea, 0x30, 0xaf, 0xfa, 0xc3, 0xff, 0xce, 0, 0xea, 0x31,
            0xaf, 0xfa, 0xc0, 0xff, 0xc6, 32, 0xea, 0x32, 0xaf, 0xfa, 0xc1, 0xff, 0xce, 0, 0xea,
            0x33, 0xaf,
        ]
    );
    // `$22C4` pumps pose/objects only when `$A28C` is nonzero.
    assert_eq!(
        &rom[0x22be..0x22ca],
        &[0xfa, 0x8c, 0xa2, 0xa7, 0x28, 0x11, 0xcd, 0x0a, 0x2d, 0xcd, 0x52, 0x06]
    );
    let direct_stores: Vec<_> = rom
        .windows(3)
        .enumerate()
        .filter_map(|(at, b)| (b == [0xea, 0x8c, 0xa2]).then_some(at))
        .collect();
    assert_eq!(direct_stores, [0x231f]);
    assert_eq!(rom[0x231e], 0xaf, "the only direct store clears the flag");
    // `$2304` lives in `$22DF`, which has no direct call/jump in this ROM.
    assert_eq!(&rom[0x2304..0x230a], &[0xcd, 0x0a, 0x2d, 0xcd, 0x52, 0x06]);
    assert!(!rom
        .windows(3)
        .any(|b| b == [0xcd, 0xdf, 0x22] || b == [0xc3, 0xdf, 0x22]));
    // Pin supported scripts rather than fabricating an opcode0/pump route.
    let scripts = &rom[0x14125..0x141e5];
    let operands = [
        Some(7),
        Some(1),
        Some(0),
        Some(2),
        Some(1),
        Some(0),
        Some(0),
        None,
        Some(0),
        None,
        Some(0),
        Some(3),
        None,
        None,
        None,
        None,
    ];
    for id in 0..32 {
        let mut pc =
            usize::from(u16::from_le_bytes([scripts[id * 2], scripts[id * 2 + 1]])) - 0x4125;
        loop {
            let op = scripts[pc];
            if op == 0xff {
                break;
            }
            assert_ne!(op >> 4, 0, "scene {id} has no descriptor/pump instruction");
            pc += 1 + operands[usize::from(op >> 4)].expect("supported instruction");
        }
    }
}
