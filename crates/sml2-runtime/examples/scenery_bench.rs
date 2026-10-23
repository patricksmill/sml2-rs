use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::content::SessionAssets;
use sml2_runtime::Session;
use std::{hint::black_box, time::Instant};
fn main() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for count in [0usize, 256, 2048, 8192] {
        let paints = (0..count)
            .map(|i| SceneryPaint {
                x: (i % 256) as i16,
                y: (i / 256 + 30) as i16,
                metatile: 1,
            })
            .collect();
        session.preview_scenery(
            (80, 448),
            SceneryOverlay::try_from_paints(paints, 128).unwrap(),
        );
        for size in [(160, 144), (384, 216)] {
            black_box(session.render_palette_frame(size));
            let mut samples = Vec::new();
            for _ in 0..7 {
                let start = Instant::now();
                black_box(session.render_palette_frame(size));
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "painted_cells={count} view={}x{} render_median_ms={:.3}",
                size.0, size.1, samples[3]
            );
        }
    }
}
