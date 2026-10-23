use sml2_core::character::Character::Mario;
use sml2_dx::Presentation;
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    session::{FrameInput, Session},
};
use std::path::PathBuf;

fn session() -> Session {
    let source =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    Session::new(SessionAssets::load_from(&source).unwrap())
}

fn save(path: &str, px: &[[u8; 3]], w: u32, h: u32) {
    let mut e = png::Encoder::new(std::fs::File::create(path).unwrap(), w, h);
    e.set_color(png::ColorType::Rgb);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&px.concat()).unwrap();
}

#[test]
#[ignore]
fn scratch_rooms() {
    let rom = std::fs::read(std::env::var_os("SML2_DX_ROM").unwrap()).unwrap();
    let dx = Presentation::new(&sml2_dx::extract(&rom).unwrap()).unwrap();
    let out = std::env::var("OUT").unwrap();
    for screen in [9u8, 10, 11, 12, 13, 14] {
        let mut g = session();
        if screen <= 12 { g.begin_coin_room(screen); } else { g.begin_bell_room(screen, 0, 0); }
        for _ in 0..30 { g.advance(FrameInput::default()); }
        for (w, tag) in [(160usize, "n"), (256, "w")] {
            let px = g.render_dx_frame(&dx, Mario, (w, 144)).unwrap();
            save(&format!("{out}/s{screen}_{tag}.png"), &px, w as u32, 144);
        }
    }
}
