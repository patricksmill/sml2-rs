use super::*;
use crate::ending_scene::EndingAssets;
use crate::ending_sequence::EndingStepContext;
use crate::object::ObjectTables;
use crate::video::PalettePixel;

/// ROM `$6844F`, written out independently of the raster module.
const SINE: [i8; 16] = [0, 5, 9, 12, 13, 12, 9, 5, 0, -5, -9, -12, -13, -12, -9, -5];

struct Ending {
    scene: EndingScene,
    sequence: EndingSequenceController,
    tables: ObjectTables,
    credits: &'static [u8],
}

impl Ending {
    fn new() -> Self {
        let assets =
            EndingAssets::from_json(include_str!("../../../../extracted/ending/scene.json"))
                .unwrap();
        let mut scene = EndingScene::default();
        let (mut state, mut fade) = (4, 0x100);
        scene.load(assets, &mut state, &mut fade);
        Self {
            scene,
            sequence: EndingSequenceController::new(),
            tables: serde_json::from_str(include_str!("../../../../extracted/objects.json"))
                .unwrap(),
            credits: include_bytes!("../../../../extracted/ending/credits.bin"),
        }
    }

    fn step(&mut self) {
        self.sequence.step(&mut EndingStepContext {
            scene: &mut self.scene,
            rom: None,
            credits_text: Some(self.credits),
            object_tables: Some(&self.tables),
            decrement_fade: true,
        });
    }

    fn run_until(&mut self, done: impl Fn(&EndingSequenceController) -> bool) {
        for _ in 0..30_000 {
            if done(&self.sequence) {
                return;
            }
            self.step();
        }
        panic!("the ending never reached the checkpoint");
    }

    fn frame(&self) -> Vec<PalettePixel> {
        render_ending_output(
            &self.scene,
            &self.sequence,
            (160, 144),
            OutputMode::PaletteIndices,
        )
        .into_iter()
        .map(PalettePixel::from_token)
        .collect()
    }

    /// The hardware shade at screen `(x, y)` from the raw tilemap and tiles,
    /// given that line's SCX and tile addressing.
    fn expected_bg(&self, x: usize, y: usize, scx: u8, unsigned: bool) -> u8 {
        let assets = self.scene.assets().unwrap();
        let (bx, by) = (
            (x as u8).wrapping_add(scx),
            (y as u8).wrapping_add(self.scene.scroll_y),
        );
        let id = assets.tilemap()[usize::from(by / 8) * 32 + usize::from(bx / 8)];
        let base = if unsigned {
            usize::from(id) * 16
        } else {
            (0x1000 + isize::from(id as i8) * 16) as usize
        };
        let row = &assets.tiles()[base + usize::from(by % 8) * 2..];
        let bit = 7 - (bx % 8);
        let color = ((row[0] >> bit) & 1) | (((row[1] >> bit) & 1) << 1);
        (self.scene.palettes.unwrap().background >> (color * 2)) & 3
    }

    /// Every background-sourced pixel of the frame matches the independent
    /// decode, line by line.
    fn assert_background(
        &self,
        frame: &[PalettePixel],
        scx: impl Fn(usize) -> u8,
        unsigned: impl Fn(usize) -> bool,
    ) {
        for y in 0..144 {
            for x in 0..160 {
                let pixel = frame[y * 160 + x];
                if pixel.source == crate::video::PaletteSource::Bg {
                    assert_eq!(
                        pixel.shade,
                        self.expected_bg(x, y, scx(y), unsigned(y)),
                        "substate {} pixel ({x}, {y})",
                        self.sequence.substate
                    );
                }
            }
        }
    }
}

fn sprite_pixels(frame: &[PalettePixel]) -> usize {
    frame
        .iter()
        .filter(|p| p.source != crate::video::PaletteSource::Bg)
        .count()
}

fn wobble(frame: u8, y: usize, raster_y: u8) -> u8 {
    if y > 16 && y <= usize::from(raster_y) {
        SINE[(usize::from((frame & 0x3C) >> 2) + y) & 15] as u8
    } else {
        0
    }
}

#[test]
fn ending_frames_draw_scene_tiles_sprites_and_per_line_raster() {
    let mut ending = Ending::new();
    // `$4520` (substates 0..3) leaves the faded-white screen up.
    ending.step();
    assert!(ending
        .frame()
        .iter()
        .all(|p| p.shade == 0 && p.source == crate::video::PaletteSource::Bg));

    // Walk (substate 5): the scene's tiles, no wobble, Mario's sprites.
    ending.run_until(|s| s.substate == 5);
    for _ in 0..40 {
        ending.step();
    }
    let walk = ending.frame();
    assert!(sprite_pixels(&walk) > 0, "the walking pair is drawn");
    ending.assert_background(&walk, |_| 0, |_| false);

    // Wobble (substate 7, once the raster has passed line 60): lines 17..=raster_y
    // take the sine SCX, the rest stay put.
    ending.run_until(|s| s.substate == 7 && s.raster_y > 60);
    let (frame, raster_y) = (ending.sequence.frame, ending.sequence.raster_y);
    let wobbling = ending.frame();
    assert!(
        (17..=usize::from(raster_y)).any(|y| wobble(frame, y, raster_y) != 0),
        "some line is displaced"
    );
    ending.assert_background(&wobbling, |y| wobble(frame, y, raster_y), |_| false);

    // `ending_request_2` has copied bank `$1A` `$5096` over the whole map.
    ending.run_until(|s| s.substate == 9);
    let assets = ending.scene.assets().unwrap();
    assert_eq!(
        Some(assets.tilemap()),
        assets.transfer_source(0x1A, 0x5096, 0x400)
    );

    // Logo and burst (substate 12): the 2x3 logo tiles are in the map and drawn.
    ending.run_until(|s| s.substate == 12);
    ending.step();
    let map = ending.scene.assets().unwrap().tilemap();
    assert_eq!(&map[10 * 32 + 8..10 * 32 + 11], &[0x89, 0x8A, 0x8B]);
    assert_eq!(&map[11 * 32 + 8..11 * 32 + 11], &[0x99, 0x9A, 0x9B]);
    for _ in 0..30 {
        ending.step();
    }
    let burst = ending.frame();
    assert!(sprite_pixels(&burst) > 0, "the burst is drawn");
    let (frame, raster_y) = (ending.sequence.frame, ending.sequence.raster_y);
    ending.assert_background(&burst, |y| wobble(frame, y, raster_y), |_| false);

    // `ending_request_1` has copied bank `$1A` `$4656` into `$8000..$87FF`.
    ending.run_until(|s| s.substate == 16);
    let assets = ending.scene.assets().unwrap();
    assert_eq!(
        Some(&assets.tiles()[..0x800]),
        assets.transfer_source(0x1A, 0x4656, 0x800)
    );

    // Credits (substate 18): scrolled 24 lines, signed tiles above line 111 and
    // unsigned below, with the text streamed into the map.
    ending.run_until(|s| s.substate == 18);
    for _ in 0..200 {
        ending.step();
    }
    assert_eq!(ending.scene.scroll_y, 24);
    let credits = ending.frame();
    let (frame, raster_y) = (ending.sequence.frame, ending.sequence.raster_y);
    ending.assert_background(&credits, |y| wobble(frame, y, raster_y), |y| y >= 111);
    assert!(sprite_pixels(&credits) > 0, "the pair keeps walking");
}

/// A sprite behind the background (attr bit 7) shows only over colour 0.
#[test]
fn ending_sprites_respect_background_priority() {
    let mut ending = Ending::new();
    ending.run_until(|s| s.substate == 5);
    ending.step();
    // Mario's first walking cell, moved over background that has both
    // colour 0 and non-zero pixels under it.
    let cell = ending.scene.objects.cells()[0];
    let colour_zero = ending.scene.palettes.unwrap().background & 3;
    let (sx, sy) = (0..136)
        .step_by(8)
        .flat_map(|y| (0..152).step_by(8).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let solid = (0..64)
                .filter(|i| ending.expected_bg(x + i % 8, y + i / 8, 0, false) != colour_zero)
                .count();
            (16..48).contains(&solid)
        })
        .expect("some 8x8 background mixes colour 0 and colours 1-3");
    let placed = crate::player::Cell {
        y: 16 + sy as u8,
        x: 8 + sx as u8,
        attr: cell.attr & !0x80,
        ..cell
    };
    let behind = crate::player::Cell {
        attr: placed.attr | 0x80,
        ..placed
    };
    ending.scene.objects.write_frame(&[placed]).unwrap();
    let front = sprite_pixels(&ending.frame());
    ending.scene.objects.write_frame(&[behind]).unwrap();
    let back = ending.frame();
    for (i, pixel) in back.iter().enumerate() {
        if pixel.source != crate::video::PaletteSource::Bg {
            let (x, y) = (i % 160, i / 160);
            assert_eq!(
                ending.expected_bg(x, y, 0, false),
                colour_zero,
                "a behind sprite pixel at ({x}, {y}) needs background colour 0"
            );
        }
    }
    assert!(sprite_pixels(&back) < front, "colours 1-3 hide it");
}
