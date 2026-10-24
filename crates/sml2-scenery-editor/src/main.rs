use sml2_scenery_editor::app::{run, Options};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut options = Options::defaults()?;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{argument} needs a value"));
        match argument.as_str() {
            "--assets" => {
                options.assets = PathBuf::from(value()?);
                options.explicit_assets = true;
            }
            "--config" => options.paths.settings = PathBuf::from(value()?),
            "--no-audio" => options.audio = false,
            "--smoke-frames" => {
                let count = value()?.parse::<u64>().map_err(|_| "--smoke-frames needs a positive count")?;
                if count == 0 { return Err("--smoke-frames needs a positive count".into()); }
                options.smoke_frames = Some(count);
            }
            "--help" | "-h" => {
                println!("sml2-scenery-editor [--assets DIR] [--config FILE] [--no-audio]\n  Validation: --smoke-frames N\n\nOptional scenery authoring companion. Uses the game's imported ROM and DX graphics.\nDrafts autosave beside settings.json; Export level writes files for a port release.\nEnter: playtest | Escape: return/cancel | B/E/I/R: tools | Ctrl+Z: undo");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {argument}")),
        }
    }
    run(options)
}
