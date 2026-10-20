//! Literal TAS input playback through a fresh native session; no emulator or save I/O.
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use serde::Deserialize;
use sml2_core::{divider::Divider, player_sim::PlayerKeys};
use sml2_runtime::session::{FrameInput, Session, SessionAssets, SessionState, StepOutcome};
use std::{
    error::Error,
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Deserialize)]
struct Movie {
    format: String,
    author: String,
    rom_sha1: String,
    frames: Vec<u8>,
}

fn keys(mask: u8) -> PlayerKeys {
    PlayerKeys {
        right: mask & 1 != 0,
        left: mask & 2 != 0,
        up: mask & 4 != 0,
        down: mask & 8 != 0,
        a: mask & 16 != 0,
        b: mask & 32 != 0,
        start: mask & 64 != 0,
        select: mask & 128 != 0,
    }
}

fn state(session: &Session) -> String {
    match &session.state {
        SessionState::Title(_) => "title".into(),
        SessionState::FileSelect(_) => "file-select".into(),
        SessionState::Demo(ctx) => format!("attract-demo-{}", ctx.level_id),
        SessionState::Overworld(_) => "map".into(),
        SessionState::InLevel(ctx) => format!("level-{}", ctx.manifest.level_id),
        SessionState::Castle(_) => "castle".into(),
        SessionState::GameOver => "game-over".into(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: tas_playback MOVIE.json [--headless] [--frames N] [--video OUT.mp4]")?;
    let mut headless = false;
    let mut limit = usize::MAX;
    let mut video = None;
    let mut codec = String::from("libx264");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--encoder" => codec = args.next().ok_or("missing ffmpeg encoder")?,
            "--frames" => limit = args.next().ok_or("missing frame limit")?.parse()?,
            "--video" => {
                video = Some(PathBuf::from(args.next().ok_or("missing video path")?));
                headless = true;
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let movie: Movie = serde_json::from_slice(&fs::read(path)?)?;
    if movie.format != "sml2-tas-v1" || movie.frames.is_empty() {
        return Err("unsupported or empty movie".into());
    }
    eprintln!(
        "TAS inputs: {} | ROM {} | {} frames",
        movie.author,
        movie.rom_sha1,
        movie.frames.len()
    );
    eprintln!("Fresh native title, one movie row per native 60 Hz tick; BIOS/lag timing is not emulated. No save files read or written.");
    let mut session = Session::new(SessionAssets::load()?);
    session.enter_title();
    let mut window = if headless {
        None
    } else {
        let mut w = Window::new(
            "Native TAS attempt",
            640,
            576,
            WindowOptions {
                resize: true,
                ..Default::default()
            },
        )?;
        w.set_target_fps(60);
        Some(w)
    };
    let mut encoder = if let Some(path) = &video {
        let author: String = movie
            .author
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '_')
            .collect();
        let filter = format!("scale=640:576:flags=neighbor,pad=640:656:0:0:black,drawtext=text='Native port - imported TAS input attempt':fontcolor=white:fontsize=18:x=10:y=582,drawtext=text='TAS inputs by {author}':fontcolor=white:fontsize=18:x=10:y=606,drawtext=text='Input frame %{{n}} - native 60 Hz':fontcolor=white:fontsize=18:x=10:y=630");
        Some(
            Command::new("ffmpeg")
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pixel_format",
                    "gray",
                    "-video_size",
                    "160x144",
                    "-framerate",
                    "60",
                    "-i",
                    "pipe:0",
                    "-vf",
                    &filter,
                    "-an",
                    "-c:v",
                    &codec,
                    "-b:v",
                    "2M",
                    "-pix_fmt",
                    "yuv420p",
                ])
                .arg(path)
                .stdin(Stdio::piped())
                .spawn()?,
        )
    } else {
        None
    };
    let mut input = encoder.as_mut().and_then(|child| child.stdin.take());
    let mut log = if let Some(path) = &video {
        let file = fs::File::create(path.with_extension("events.jsonl"))?;
        Some(file)
    } else {
        None
    };
    let mut previous = String::new();
    let mut buf = Vec::new();
    let mut index = 0;
    let count = limit.min(movie.frames.len());
    let mut paused = false;
    while index < count {
        let step = if let Some(w) = &mut window {
            if !w.is_open() || w.is_key_down(Key::Escape) {
                break;
            }
            if w.is_key_pressed(Key::Space, KeyRepeat::No) {
                paused = !paused;
            }
            !paused || w.is_key_pressed(Key::Right, KeyRepeat::No)
        } else {
            true
        };
        if step {
            let tick = index as u64 + 1;
            let elapsed = Duration::new(tick / 60, ((tick % 60) * 1_000_000_000 / 60) as u32);
            let effects = session.advance(FrameInput {
                keys: keys(movie.frames[index]),
                div: Divider::from_elapsed(0, elapsed).sample(),
            });
            let current = state(&session);
            if current != previous || effects.outcome != StepOutcome::Continue {
                eprintln!("frame {index}: {current} {:?}", effects.outcome);
                if let Some(file) = &mut log {
                    writeln!(
                        file,
                        "{}",
                        serde_json::json!({"frame": index, "state": current, "outcome": format!("{:?}", effects.outcome)})
                    )?;
                }
                previous = current;
            }
            for diagnostic in effects.diagnostics {
                eprintln!("frame {index}: {diagnostic}");
            }
            if let Some(pipe) = &mut input {
                pipe.write_all(&session.render_frame((160, 144)))?;
            }
            index += 1;
        }
        if let Some(w) = &mut window {
            w.set_title(&format!(
                "TAS inputs by {} | frame {index}/{} | {} | Space pause, Right step, Esc exit",
                movie.author,
                movie.frames.len(),
                state(&session)
            ));
            let (width, height) = w.get_size();
            if width > 0 && height > 0 {
                sml2_runtime::present::letterbox_gray(
                    &session.render_frame((160, 144)),
                    (160, 144),
                    (width, height),
                    &mut buf,
                );
                w.update_with_buffer(&buf, width, height)?;
            } else {
                w.update();
            }
        }
    }
    drop(input);
    if let Some(mut child) = encoder {
        if !child.wait()?.success() {
            return Err("ffmpeg video export failed".into());
        }
    }
    eprintln!("Played {index}/{} input frames; final native state: {}. Playback is an attempt, not a completion assertion.", movie.frames.len(), state(&session));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_masks_hold_simultaneous_buttons_without_reordering() {
        let k = keys(1 | 16 | 32 | 64);
        assert!(k.right && k.a && k.b && k.start);
        assert!(!k.left && !k.up && !k.down && !k.select);
        let k = keys(2 | 4 | 8 | 128);
        assert!(k.left && k.up && k.down && k.select);
        assert!(!k.right && !k.a && !k.b && !k.start);
    }
}
