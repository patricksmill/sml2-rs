//! Pipe host for tools/compare_tas.py: initial frame, then one frame per mask line.
//! Stdout is raw 160x144 grayscale; diagnostics go to stderr. No save-file I/O.
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
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--initial-phase" => {
                let phase: u8 = args.next().ok_or("missing initial phase")?.parse()?;
                if phase > 7 {
                    return Err("initial phase must be 0..7".into());
                }
                initial_phase = Some(phase);
            }
            _ => return Err(format!("unknown bridge argument: {arg}").into()),
        }
    }
    let mut session = Session::new(SessionAssets::load()?);
    session.enter_title();
    let mut out = io::BufWriter::new(io::stdout().lock());
    out.write_all(&session.render_frame((160, 144)))?;
    out.flush()?;
    for (index, line) in io::stdin().lock().lines().enumerate() {
        let mask: u8 = line?.parse()?;
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
        out.write_all(&session.render_frame((160, 144)))?;
        out.flush()?;
    }
    Ok(())
}
