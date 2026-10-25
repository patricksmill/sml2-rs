//! Exercise native resize/fullscreen presentation without game data or audio.
use eframe::egui::{self, Color32, ViewportCommand};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

struct Smoke {
    start: Instant,
    step: usize,
    completed: Arc<AtomicUsize>,
}

impl eframe::App for Smoke {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Graphics resize / fullscreen smoke test");
            ui.label(format!("Transition {}", self.step));
            ui.painter()
                .rect_filled(ui.available_rect_before_wrap(), 0, Color32::DARK_BLUE);
        });
        if self.start.elapsed() >= Duration::from_millis(250 * (self.step + 1) as u64) {
            eprintln!("Graphics transition {}", self.step);
            match self.step {
                0 | 4 | 8 => {
                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(1200.0, 800.0)))
                }
                1 | 5 | 9 => ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true)),
                2 | 6 | 10 => ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false)),
                3 | 7 | 11 => {
                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(480.0, 400.0)))
                }
                _ => {
                    eprintln!("Graphics smoke test completed all transitions");
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
            self.step += 1;
            self.completed.store(self.step, Ordering::Relaxed);
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn main() -> Result<(), String> {
    let completed = Arc::new(AtomicUsize::new(0));
    let result = completed.clone();
    eframe::run_native(
        "SML2 graphics smoke test",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 720.0])
                .with_min_inner_size([480.0, 400.0]),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: sml2_desktop::ui::graphics::configuration(),
            ..Default::default()
        },
        Box::new(move |cc| {
            if let Some(render) = &cc.wgpu_render_state {
                eprintln!("Graphics adapter: {:?}", render.adapter.get_info());
            }
            Ok(Box::new(Smoke {
                start: Instant::now(),
                step: 0,
                completed,
            }))
        }),
    )
    .map_err(|err| err.to_string())?;
    let count = result.load(Ordering::Relaxed);
    if count < 13 {
        return Err(format!("Graphics test exited after {count} transitions"));
    }
    Ok(())
}
