//! Host-independent castle transitions; all input is edge-triggered joypad data.
use super::*;
use sml2_core::castle_sequence::{CastleTransition, SequenceInput, LEVEL_PROGRESS_OFFSETS};

impl Session {
    pub(super) fn advance_castle(
        &mut self,
        pressed: u8,
        saves: &mut Vec<[u8; 0x50]>,
        diagnostics: &mut Vec<String>,
    ) -> StepOutcome {
        let SessionState::Castle(seq) = &mut self.state else {
            unreachable!()
        };
        let offset = LEVEL_PROGRESS_OFFSETS
            .get(seq.level as usize)
            .copied()
            .unwrap_or(0) as usize;
        let out = seq.tick(
            SequenceInput {
                keys_pressed: pressed,
                frame: self.frame_n as u8,
            },
            offset,
            &mut self.castle_progress,
        );
        self.level_tally = seq.level_tally;
        if let Some(music) = out.music {
            self.sound_driver.music_request = music;
        }
        if let Some(sfx) = out.sfx2 {
            self.sound_driver.sfx_request_2 = sfx;
        }
        match out.transition {
            Some(CastleTransition::EnterLevel(id)) => {
                // $46C7 -> $3573: level 24 now follows the ordinary level loader.
                let Some((_, node, _, _)) = self.castle_return else {
                    unreachable!()
                };
                let castle = seq.clone();
                if let Err(error) = self.begin_level(id, 0) {
                    diagnostics.push(error);
                    return StepOutcome::Continue;
                }
                if let SessionState::InLevel(ctx) = &mut self.state {
                    ctx.sim.map_node = node;
                    ctx.sim.level_progress = self.castle_progress;
                    ctx.sim.level_tally = self.level_tally;
                }
                self.castle_return = None;
                if self.easy_mode {
                    // `$3573` runs the card first: the frame after `$16`
                    // starts `$35CD`, and the level loads once it ends.
                    let level = std::mem::replace(&mut self.state, SessionState::Castle(castle));
                    let SessionState::InLevel(ctx) = level else {
                        unreachable!("begin_level loaded the level")
                    };
                    self.pending_level = Some(Box::new(MapTransition {
                        ctx,
                        level_id: id,
                        frame: 0,
                        easy_card: Some(1),
                        live_at: 1 + sml2_core::easy_mode_card::EASY_CARD_FRAMES,
                    }));
                    return StepOutcome::Continue;
                }
                StepOutcome::LevelEntered(id)
            }
            Some(CastleTransition::ReturnToMap { walked_out }) => {
                if let Some((screen, node, flag, direction)) = self.castle_return.take() {
                    let mut map = MapSim::new();
                    map.set_castle_scripts(self.assets.castle_scripts.clone());
                    map.lives = self.lives;
                    map.powerup = self.powerup;
                    map.coins_low = self.coins_low;
                    map.coins_high = self.coins_high;
                    map.level_progress = self.castle_progress;
                    map.load_screen(screen, flag, &self.assets.node_records);
                    map.node = node;
                    map.load_node(node, &self.assets.node_records);
                    if let Some(direction) = direction {
                        map.cursor_move = direction;
                    }
                    if walked_out {
                        map.mario_dead = 1;
                    }
                    self.state = SessionState::Overworld(map);
                    let record = self.save_record();
                    if let Some(slot) = self.active_slot {
                        self.slots.write_slot(slot as usize, &record);
                    }
                    saves.push(record);
                }
                StepOutcome::Continue
            }
            Some(CastleTransition::Reset) => {
                self.castle_return = None;
                self.state = SessionState::Title(TitleScreen::new());
                self.active_slot = None;
                self.sound_driver.music_request = 0xFF;
                StepOutcome::Continue
            }
            None => StepOutcome::Continue,
        }
    }
}
