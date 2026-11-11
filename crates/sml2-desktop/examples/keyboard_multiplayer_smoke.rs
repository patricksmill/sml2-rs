//! Exercise the desktop keyboard sampler and game host without physical devices.
//! Run from the repository root; optionally pass an output PNG path.
use sml2_desktop::{
    game::Game,
    input::{Bindings, Button, InputState},
};
use sml2_runtime::session::{SessionAssets, SessionState};

fn positions(game: &Game) -> [(u16, u16); 2] {
    let SessionState::InLevel(level) = &game.session.state else {
        panic!("expected live level");
    };
    let second = level.second.as_ref().expect("second player");
    [(level.sim.x, level.sim.y), (second.sim.x, second.sim.y)]
}

fn tick(game: &mut Game, input: &mut InputState, bindings: &[Bindings; 2]) {
    let (first, second) = input.sample_two_players(&bindings[0], &bindings[1]);
    game.tick_two_players(first.keys, second.keys, 0.0);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut game = Game::preview(SessionAssets::load()?, false);
    game.session.set_local_multiplayer(true);
    let bindings = [Bindings::default(), Bindings::player_two()];
    let mut input = InputState::default();

    game.session.begin_level(0, 0)?;
    let start = positions(&game);
    input.event(Button::key("ArrowRight"), true);
    input.event(Button::key("F"), true);
    for _ in 0..20 {
        tick(&mut game, &mut input, &bindings);
    }
    let moved = positions(&game);
    assert!(moved[0].0 > start[0].0, "P1 Right must move P1");
    assert_eq!(moved[1].0, start[1].0, "P2 jump must not move horizontally");
    assert!(moved[1].1 < start[1].1, "P2 F must jump");
    println!("P1 Right + P2 F: {start:?} -> {moved:?}");

    // Focus loss clears both players, including held and queued tap input.
    input.clear();
    let (first, second) = input.sample_two_players(&bindings[0], &bindings[1]);
    assert!(!first.keys.right && !second.keys.a);
    game.session.begin_level(0, 0)?;
    let start = positions(&game);
    input.event(Button::key("D"), true);
    for _ in 0..20 {
        tick(&mut game, &mut input, &bindings);
    }
    let moved = positions(&game);
    assert_eq!(moved[0].0, start[0].0, "P2 D must not move P1");
    assert!(moved[1].0 > start[1].0, "P2 D must move P2");
    println!("P2 D only: {start:?} -> {moved:?}");

    input.suppress_until_release();
    let (_, second) = input.sample_two_players(&bindings[0], &bindings[1]);
    assert!(!second.keys.right, "pause must suppress held P2 input");
    input.event(Button::key("D"), false);
    input.event(Button::key("D"), true);
    let (_, second) = input.sample_two_players(&bindings[0], &bindings[1]);
    assert!(second.keys.right, "release/repress must restore P2 input");
    println!("focus loss, pause suppression, and release/repress: ok");

    if let Some(path) = std::env::args_os().nth(1) {
        let pixels = game.session.render_frame((160, 144));
        let rgb: Vec<u8> = pixels.iter().flat_map(|&p| [p; 3]).collect();
        let mut encoder = png::Encoder::new(std::fs::File::create(&path)?, 160, 144);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&rgb)?;
        println!(
            "two-player CPU frame: {}",
            std::path::Path::new(&path).display()
        );
    }
    assert!(game.diagnostics.is_empty(), "{:?}", game.diagnostics);
    println!("keyboard_multiplayer_smoke: passed");
    Ok(())
}
