//! Wario's second-form exit prerequisite, ROM `$AC55..$ACB1`.

use crate::actor::{Actor, AnimationFrameError, FrameTick};

/// Source writes the fade low byte before invoking the animation helpers.
const FADE_LOW: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExitEffects {
    replacement_animation: Option<u8>,
}

/// The raw difficulty input tests bit zero only; unrelated flags do not select easy.
fn update_exit(
    actor: &mut Actor,
    frame: FrameTick,
    easy_mode_flags: u8,
) -> Result<ExitEffects, AnimationFrameError> {
    actor.animate_four_step_bob(frame)?;
    actor.flash.animate_palette(frame);
    actor.screen.move_x(2);
    let replacement_animation = if actor.screen.x >= 192 {
        *actor = Actor::default();
        Some(if easy_mode_flags & 1 == 0 { 27 } else { 29 })
    } else {
        None
    };
    Ok(ExitEffects {
        replacement_animation,
    })
}

// Compatibility adapter: owns fade/event writes and the legacy stream boundary.
use crate::objects::{ObjectSim, ObjectTables, SpawnRecord, Staging};

impl ObjectSim {
    pub(in crate::objects) fn wario_phase2_exit(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
    ) {
        self.fade_timer = (self.fade_timer & 0xFF00) | u16::from(FADE_LOW);
        self.fade_timer_set = Some(self.fade_timer);
        let mut actor = Actor::from_raw_slot(&st.b);
        let effects = update_exit(
            &mut actor,
            FrameTick(u64::from(self.fff0)),
            u8::from(self.easy_mode),
        );
        st.b = actor.to_raw_slot();
        let effects = effects.expect("Wario exit cycle must address the four-entry ROM table");
        if let Some(animation) = effects.replacement_animation {
            self.done = 1; // $B15B blanks without decrementing the definition count.
            self.scene_phase_set = Some(2);
            self.sprite_map_index = 1;
            let sprite_id = tables.sprite_for(0x0A, animation).unwrap_or(0);
            let hitbox = tables.hitbox_for(0x0A, sprite_id).unwrap_or([0; 4]);
            // Six left sentinels and the right $FF marker are abstracted by records.
            // Cursor zero denotes the ROM's $AB06, immediately after the sentinels.
            self.records = vec![SpawnRecord {
                x: 0x08C8,
                y: 0x00B0,
                anim_index: animation,
                mode_flag: 0,
                sprite_id,
                hitbox,
                raw: [8, 0xC8, 0, 0, 0xB0, animation],
            }];
            self.seed_stream_cursor();
            self.spawned = vec![false];
        }
    }
}

#[cfg(test)]
#[path = "tests/phase2_exit.rs"]
mod tests;
