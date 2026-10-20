//! Pipe host for tools/compare_tas.py: initial frame, then one frame per mask line.
//! Stdout is raw 160x144 grayscale; diagnostics go to stderr. No save-file I/O.
//! With `--state`, every frame (the first included) is preceded by a
//! `STATE level x y camx camy` line so the viewer can compare native positions
//! against the emulator and stop at the first divergence. `-` marks a frame
//! outside a level.
use sml2_core::{divider::Divider, player_sim::PlayerKeys};
use sml2_runtime::session::{FrameInput, Session, SessionAssets};
use std::{
    error::Error,
    io::{self, BufRead, Write},
    time::Duration,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut initial_phase = None;
    let mut state_lines = false;
    let mut phase_lock = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--initial-phase" => {
                let phase: u8 = args.next().ok_or("missing initial phase")?.parse()?;
                if phase > 7 {
                    return Err("initial phase must be 0..7".into());
                }
                initial_phase = Some(phase);
            }
            "--state" => state_lines = true,
            "--phase-lock" => phase_lock = true,
            _ => return Err(format!("unknown bridge argument: {arg}").into()),
        }
    }
    let mut session = Session::new(SessionAssets::load()?);
    session.enter_title();
    let mut out = io::BufWriter::new(io::stdout().lock());
    write_frame(&mut out, &session, state_lines)?;
    for (index, line) in io::stdin().lock().lines().enumerate() {
        // `--phase-lock` rows are `mask phase`: the ROM's own `$FF97 & 7`, which
        // a frame-synchronous port cannot derive (it counts vblank waits from
        // boot). Only the comparison harness supplies it.
        let line = line?;
        let mut fields = line.split_whitespace();
        let mask: u8 = fields.next().ok_or("missing input mask")?.parse()?;
        let rom_phase: Option<u8> = match fields.next() {
            Some(value) if phase_lock => Some(value.parse()?),
            _ => None,
        };
        let keys = PlayerKeys {
            right: mask & 1 != 0,
            left: mask & 2 != 0,
            up: mask & 4 != 0,
            down: mask & 8 != 0,
            a: mask & 16 != 0,
            b: mask & 32 != 0,
            start: mask & 64 != 0,
            select: mask & 128 != 0,
        };
        let tick = index as u64 + 1;
        let elapsed = Duration::new(tick / 60, ((tick % 60) * 1_000_000_000 / 60) as u32);
        // Explicit comparison origin, applied once before the first gameplay
        // update. This is not a fix for the session's missing shared frame clock.
        if let (Some(phase), sml2_runtime::session::SessionState::InLevel(ctx)) =
            (rom_phase, &mut session.state)
        {
            // `step_frame` increments before the player update, so pre-load one
            // step short of the ROM's value.
            ctx.sim.phase = phase.wrapping_sub(1) & 7;
        }
        if let sml2_runtime::session::SessionState::InLevel(ctx) = &mut session.state {
            if ctx.frame_n == 0 {
                if let Some(phase) = initial_phase.take() {
                    ctx.sim.phase = phase;
                    eprintln!("comparison: first level initial movement phase = {phase}");
                }
            }
        }
        let effects = session.advance(FrameInput {
            keys,
            div: Divider::from_elapsed(0, elapsed).sample(),
        });
        if effects.outcome != sml2_runtime::session::StepOutcome::Continue {
            eprintln!("input frame {index}: {:?}", effects.outcome);
        }
        for diagnostic in effects.diagnostics {
            eprintln!("{diagnostic}");
        }
        write_frame(&mut out, &session, state_lines)?;
    }
    Ok(())
}

/// `STATE level x y camx camy`, or dashes outside a level, then the pixels.
fn write_frame(
    out: &mut impl Write,
    session: &Session,
    state_lines: bool,
) -> Result<(), Box<dyn Error>> {
    if state_lines {
        let line = match &session.state {
            sml2_runtime::session::SessionState::InLevel(c) => format!(
                "STATE {} {} {} {} {} {:02x} {:02x} {} {}",
                c.manifest.level_id, c.sim.x, c.sim.y, c.cam.x, c.cam.y,
                c.sim.hvel, c.sim.vvel, u8::from(c.sim.on_ground), c.sim.phase
            ),
            _ => "STATE - - - - -".to_string(),
        };
        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
    }
    out.write_all(&session.render_frame((160, 144)))?;
    out.flush()?;
    Ok(())
}
