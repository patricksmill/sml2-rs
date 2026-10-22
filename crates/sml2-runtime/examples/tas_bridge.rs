//! Pipe host for tools/compare_tas.py: initial frame, then one frame per mask line.
//! Stdout is raw 160x144 grayscale; diagnostics go to stderr. No save-file I/O.
//! With `--state`, every frame (the first included) is preceded by a
//! `STATE level x y camx camy hvel vvel on_ground phase` line (velocities are
//! hexadecimal; other fields are decimal) so the viewer can compare native positions
//! against the emulator and stop at the first divergence. `-` marks a frame
//! outside a level. `--debug-state` appends decimal `key=value` diagnostics to
//! that same line, preserving the following pixel frame boundary.
use sml2_core::player_sim::PlayerKeys;
use sml2_runtime::{
    divider_clock::VirtualDividerClock,
    session::{FrameInput, Session, SessionAssets},
};
use std::{
    error::Error,
    io::{self, BufRead, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut initial_phase = None;
    let mut state_lines = false;
    let mut phase_lock = false;
    let mut debug_state = false;
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
            "--debug-state" => {
                state_lines = true;
                debug_state = true;
            }
            "--phase-lock" => phase_lock = true,
            _ => return Err(format!("unknown bridge argument: {arg}").into()),
        }
    }
    let mut session = Session::new(SessionAssets::load()?);
    session.enter_title();
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut div_clock = VirtualDividerClock::new(0);
    write_frame(&mut out, &session, state_lines, debug_state, 0)?;
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
        // Optional `f97=N div=a,b,...`: the console's whole `$FF97` and the
        // frame's `rDIV` reads, so the map's shared random byte can follow it.
        let mut rom_timing: Option<sml2_runtime::session::RomTiming> = None;
        let mut boss_div: Option<u8> = None;
        for field in fields {
            // Comparison-only environmental input: the witch reads DIV once
            // when choosing her side or fire trajectory. Keep it separate
            // from the map's ordered random-read stream and frame clock.
            if let Some(value) = field.strip_prefix("boss_div=") {
                boss_div = Some(value.parse()?);
                continue;
            }
            let timing = rom_timing.get_or_insert_with(Default::default);
            match field.split_once('=') {
                Some(("f97", value)) => timing.ff97 = value.parse()?,
                Some(("div", "")) => {}
                Some(("div", values)) => {
                    for value in values.split(',') {
                        timing.div_reads.push(value.parse()?);
                    }
                }
                _ => return Err(format!("unknown bridge field {field:?}").into()),
            }
        }
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
        div_clock.advance_tick();
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
        if let Some(timing) = rom_timing {
            session.set_rom_timing(timing);
        }
        let effects = session.advance(FrameInput {
            keys,
            div: boss_div.unwrap_or_else(|| div_clock.sample()),
        });
        if effects.outcome != sml2_runtime::session::StepOutcome::Continue {
            eprintln!("input frame {index}: {:?}", effects.outcome);
        }
        for diagnostic in effects.diagnostics {
            eprintln!("{diagnostic}");
        }
        write_frame(&mut out, &session, state_lines, debug_state, tick)?;
    }
    Ok(())
}

/// `STATE level x y camx camy hvel vvel on_ground phase`, or five dashes outside
/// a level, then the pixels. Velocities are hexadecimal; other fields are decimal.
fn write_frame(
    out: &mut impl Write,
    session: &Session,
    state_lines: bool,
    debug_state: bool,
    native_step: u64,
) -> Result<(), Box<dyn Error>> {
    if state_lines {
        let mut line = match &session.state {
            sml2_runtime::session::SessionState::InLevel(c) => format!(
                "STATE {} {} {} {} {} {:02x} {:02x} {} {}",
                c.manifest.level_id,
                c.sim.x,
                c.sim.y,
                c.cam.x,
                c.cam.y,
                c.sim.hvel,
                c.sim.vvel,
                u8::from(c.sim.on_ground),
                c.sim.phase
            ),
            _ => "STATE - - - - -".to_string(),
        };
        if debug_state {
            if let sml2_runtime::session::SessionState::InLevel(c) = &session.state {
                line.push_str(&format!(
                    " step={} fff0={} update_phase={} standing={} scene={} owns={} pc={}",
                    native_step,
                    c.obj.fff0,
                    c.obj.update_phase,
                    u8::from(c.sim.on_ground_obj),
                    c.cam.scene_id,
                    u8::from(c.scene_vm.owns_gameplay()),
                    c.scene_vm.pc
                ));
                let boss = &c.obj.slots[0].b;
                let platform = &c.obj.slots[1].b;
                line.push_str(&format!(
                    " boss_x={} boss_state={} door_phase={} door_parity={} platform_sprite={} tile_anim={} writes={} tile_timer={} tile_phase={} ff97={} bag={} pow={} wt={}",
                    boss[10], boss[15], c.obj.door_phase, c.obj.door_parity,
                    platform[11], c.sim.tile_anim_config, c.sim.layout_writes.len(),
                    c.tile_animation.timer, c.tile_animation.phase, session.ff97(),
                    c.obj.money_bag, c.sim.powerup, c.sim.walk_timer
                ));
            } else {
                let clock = session.tile_animation_clock();
                line.push_str(&format!(
                    " tile_timer={} tile_phase={} ff97={}",
                    clock.timer,
                    clock.phase,
                    session.ff97()
                ));
                if let sml2_runtime::session::SessionState::Overworld(map) = &session.state {
                    // sMapFrameCounter ($A69A).
                    line.push_str(&format!(" map_frame={}", map.frame_counter));
                }
            }
        }
        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
    }
    out.write_all(&session.render_frame((160, 144)))?;
    out.flush()?;
    Ok(())
}
