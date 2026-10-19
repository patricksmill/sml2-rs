use sml2_desktop::app::{self, Options};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut options = Options::defaults()?;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--assets" => options.assets = PathBuf::from(value()?),
            "--config" => options.paths.settings = PathBuf::from(value()?),
            "--save" => options.paths.save = PathBuf::from(value()?),
            "--no-audio" => options.audio = false,
            "--play" => options.play = true,
            "--fullscreen" => options.fullscreen = true,
            "--scale" => {
                let scale: u8 = value()?.parse().map_err(|_| "--scale needs 1..=8")?;
                if !(1..=8).contains(&scale) {
                    return Err("--scale needs 1..=8".into());
                }
                options.scale = Some(scale);
            }
            "--smoke-frames" => {
                let frames: u64 = value()?
                    .parse()
                    .map_err(|_| "--smoke-frames needs a positive count")?;
                if frames == 0 {
                    return Err("--smoke-frames needs a positive count".into());
                }
                options.smoke_frames = Some(frames);
            }
            "--screenshot" => options.screenshot = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                println!("sml2-desktop [--assets DIR] [--config FILE] [--save FILE] [--play]\n  [--scale 1..8] [--fullscreen] [--no-audio]\n  Validation: --smoke-frames N [--screenshot FILE]\n\nEsc / Controller Start: menu | F11: fullscreen | +/-: zoom\nF1: debug | F2: debug pause | F3: one tick");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {argument}")),
        }
    }
    if options.screenshot.is_some() && options.smoke_frames.is_none() {
        return Err("--screenshot requires --smoke-frames".into());
    }
    app::run(options)
}
