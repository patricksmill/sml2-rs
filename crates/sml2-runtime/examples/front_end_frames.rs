//! Writes the front-end screens as PGM frames for visual inspection.
use sml2_core::player_sim::PlayerKeys;
use sml2_runtime::session::{FrameInput, Session, SessionAssets};

fn write_pgm(name: &str, frame: &[u8]) {
    let mut out = format!("P5\n160 144\n255\n").into_bytes();
    out.extend_from_slice(frame);
    std::fs::write(name, out).unwrap();
}

fn main() {
    let assets = SessionAssets::load().unwrap();
    let mut session = Session::new(assets);
    session.enter_title();
    write_pgm("/tmp/fe_title.pgm", &session.render_frame((160, 144)));

    let step = |session: &mut Session, keys: PlayerKeys| {
        session.advance(FrameInput { keys, div: 0 });
    };
    for _ in 0..64 {
        step(&mut session, PlayerKeys::default());
    }
    step(
        &mut session,
        PlayerKeys {
            start: true,
            ..Default::default()
        },
    );
    for _ in 0..46 {
        step(&mut session, PlayerKeys::default());
    }
    write_pgm("/tmp/fe_select.pgm", &session.render_frame((160, 144)));

    step(
        &mut session,
        PlayerKeys {
            select: true,
            ..Default::default()
        },
    );
    write_pgm("/tmp/fe_easy.pgm", &session.render_frame((160, 144)));

    // `$0B`: run the first attract demo and capture a mid-level frame.
    assert!(session.start_demo(0));
    for _ in 0..180 {
        step(&mut session, PlayerKeys::default());
    }
    write_pgm("/tmp/fe_demo.pgm", &session.render_frame((160, 144)));
}
