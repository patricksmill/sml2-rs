//! Native zone presentation state, updated after `$3C13E` and before the cursor.
use super::MapSim;

#[path = "animation_growth.rs"]
mod animation_growth;
#[path = "animation_mario.rs"]
mod animation_mario;
#[path = "animation_patches.rs"]
mod animation_patches;
#[path = "animation_space.rs"]
mod animation_space;
#[path = "animation_tiles.rs"]
mod animation_tiles;
#[path = "animation_tree.rs"]
mod animation_tree;
#[path = "animation_turtle.rs"]
mod animation_turtle;
#[path = "animation_wario_castle.rs"]
mod animation_wario_castle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneAnimation {
    tree: animation_tree::TreeAnimation,
    space: animation_space::SpaceAnimation,
    wario_castle: animation_wario_castle::WarioCastleAnimation,
    moving_x: u8,
    moving_direction: u8,
    pub(super) random_accum: u8,
    pub(super) global_frame: u8,
    sampled_timer: u8,
    transition_state: u8,
    pub(super) sfx0: u8,
    patch_ids: [u8; 16],
    patch_positions: [(u8, u8); 16],
    tile_writes: Vec<Option<u8>>,
    render_slots: Option<[(u8, u8, u8); 16]>,
    scroll_x: u8,
    scroll_y: u8,
    growth_active: bool,
    tiles: animation_tiles::TileAnimation,
}

impl Default for ZoneAnimation {
    fn default() -> Self {
        Self {
            tree: Default::default(),
            space: Default::default(),
            wario_castle: Default::default(),
            moving_x: 0,
            moving_direction: 0,
            random_accum: 0,
            global_frame: 0,
            sampled_timer: 0,
            transition_state: 0,
            sfx0: 0,
            patch_ids: [255; 16],
            patch_positions: [(0, 0); 16],
            tile_writes: vec![None; 1024],
            render_slots: None,
            scroll_x: 0,
            scroll_y: 0,
            growth_active: false,
            tiles: Default::default(),
        }
    }
}

impl ZoneAnimation {
    fn apply_patches(&mut self) {
        for (&id, &(x, y)) in self.patch_ids.iter().zip(&self.patch_positions) {
            if let Some(patch) = animation_patches::PATCHES.get(id as usize) {
                for &(dx, dy, tile) in *patch {
                    let at = (y.wrapping_add(dy) as usize & 31) * 32
                        + (x.wrapping_add(dx) as usize & 31);
                    self.tile_writes[at] = Some(tile);
                }
            }
        }
    }
}

impl MapSim {
    pub fn apply_zone_tile_animation(&self, vram: &mut [[u8; 64]], source: &[u8]) -> bool {
        self.zone_animation
            .tiles
            .apply(self.screen_id, vram, source)
    }
    /// `$FF97` is free-running across screen loads; native hosts supply it.
    pub fn set_zone_animation_clock(&mut self, frame: u8, timer: u8) {
        self.zone_animation.global_frame = frame;
        self.zone_animation.sampled_timer = timer;
    }

    pub fn zone_render_slots(&self) -> &[(u8, u8, u8); 16] {
        self.zone_animation
            .render_slots
            .as_ref()
            .unwrap_or(&self.zone_slots)
    }

    pub fn zone_tile_writes(&self) -> Vec<(u16, u8)> {
        let mut writes: Vec<(u16, u8)> = self
            .zone_animation
            .tile_writes
            .iter()
            .enumerate()
            .filter_map(|(i, &t)| t.map(|t| (i as u16, t)))
            .collect();
        // Screens 9-12: `$3EBD5` redraws the prize panel and the counters.
        if let Some(room) = self
            .room
            .as_ref()
            .filter(|_| (9..=12).contains(&self.screen_id))
        {
            writes.extend(room.bg_writes(self.node, self.coins_low, self.coins_high, self.lives));
        }
        writes
    }

    pub fn zone_background_scroll(&self, row: usize) -> (u8, u8) {
        if self.screen_id == 7 {
            let band = match row {
                0..=14 => Some(0),
                15..=22 => Some(1),
                23..=30 => Some(2),
                _ => None,
            };
            return (
                band.map_or(0, |i| self.zone_animation.wario_castle.band_scroll[i]),
                0,
            );
        }
        (self.zone_animation.scroll_x, self.zone_animation.scroll_y)
    }

    pub fn zone_palettes(&self) -> Option<(u8, u8)> {
        (self.screen_id == 7).then_some((
            self.zone_animation.wario_castle.bg_palette,
            self.zone_animation.wario_castle.obj_palette,
        ))
    }

    pub(super) fn zone_animation_tick(&mut self, div: u8) {
        self.zone_animation.render_slots = None;
        self.zone_animation.sfx0 = 0;
        animation_tiles::tick(self, div);
        match self.screen_id {
            1 | 3 => animation_growth::tick(self),
            2 => animation_tree::tick(self, div),
            4 => animation_space::tick(self, div),
            5 => animation_turtle::tick(self, div),
            6 => animation_mario::tick(self, div),
            7 => animation_wario_castle::tick(self, div),
            8 => animation_growth::hill_tick(self),
            _ => {}
        }
        self.zone_animation.apply_patches();
    }

    pub(super) fn reset_zone_animation(
        &mut self,
        screen: u8,
        fresh_exit: bool,
        old_progress: [u8; 42],
    ) {
        self.zone_animation.patch_ids = [255; 16];
        self.zone_animation.patch_positions = [(0, 0); 16];
        self.zone_animation.tile_writes.fill(None);
        self.zone_animation.render_slots = None;
        self.zone_animation.scroll_x = 0;
        self.zone_animation.scroll_y = 0;
        self.zone_animation.growth_active = false;
        animation_tiles::reset(self);
        match screen {
            1 => {
                // Saved branches are painted completely before a fresh exit
                // starts its incremental growth at patch 31 or 39.
                for (flag, first, end) in [(7, 31, 39), (8, 39, 47)] {
                    if old_progress[flag] & 1 != 0 {
                        for id in first..end {
                            self.zone_animation.patch_ids[0] = id;
                            self.zone_animation.apply_patches();
                        }
                    }
                }
                if fresh_exit {
                    let branch = match self.node {
                        44 => Some((0, 7, 31)),
                        47 => Some((1, 8, 39)),
                        _ => None,
                    };
                    if let Some((slot, flag, first)) = branch {
                        self.zone_animation.patch_ids[slot] = first;
                        self.zone_animation.growth_active = old_progress[flag] & 1 == 0;
                    }
                }
            }
            2 => {
                animation_tree::reset(self);
                self.zone_slots[9].2 = if old_progress[2] & 1 != 0 {
                    if old_progress[36] & 0x80 != 0 {
                        124
                    } else {
                        109
                    }
                } else if fresh_exit {
                    106
                } else {
                    255
                };
            }
            4 => {
                animation_space::reset(self);
                if old_progress[16] & 1 != 0 {
                    self.zone_animation.patch_ids[0] = 56;
                    self.zone_animation.apply_patches();
                    self.zone_animation.patch_ids[0] = 51;
                    self.zone_animation.patch_ids[1] = 52;
                    self.zone_animation.apply_patches();
                    if old_progress[41] & 0x80 != 0 {
                        self.zone_animation.patch_ids[0] = 57;
                    }
                }
                if fresh_exit {
                    if old_progress[16] & 1 == 0 {
                        animation_space::start_launch(self);
                        // The fresh loader's sprite list still hides the UFO
                        // until its first odd-frame launch update.
                        self.zone_slots[7].2 = 255;
                    }
                    self.zone_animation.patch_ids[0] = 47;
                }
            }
            3 => {
                if old_progress[11] & 1 != 0 {
                    for id in 16..31 {
                        self.zone_animation.patch_ids[0] = id;
                        self.zone_animation.apply_patches();
                    }
                }
                if fresh_exit && self.node == 107 {
                    self.zone_animation.patch_ids[0] = 16;
                    self.zone_animation.growth_active = old_progress[11] & 1 == 0;
                }
            }
            6 => {
                self.zone_animation.patch_positions[..4].copy_from_slice(&[
                    (4, 7),
                    (14, 7),
                    (8, 9),
                    (8, 7),
                ]);
                self.zone_animation.patch_ids[..4].copy_from_slice(&[0, 3, 6, 9]);
                self.zone_animation.apply_patches();
                self.zone_animation.patch_ids[2] = 6 + ((self.level_progress[27] & 0x80) >> 6);
                self.zone_animation.patch_ids[3] = 9 + ((self.level_progress[28] & 0x80) >> 6);
            }
            7 => animation_wario_castle::reset(self),
            5 => animation_turtle::reset(self),
            _ => {}
        }
        self.zone_animation.apply_patches();
    }
}

#[cfg(test)]
#[path = "zone_animation_tests.rs"]
mod tests;
