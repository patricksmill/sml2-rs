//! Object walkers routines.

use super::*;

/// Probe the level for a native actor: the slot's position is synced first,
/// since the ROM probes read the slot after any move.
struct SimWalkerWorld<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl<'a> SimWalkerWorld<'a> {
    fn new(sim: &'a mut ObjectSim, st: &'a mut Staging, layout: &'a LevelLayout) -> Self {
        Self { sim, st, layout }
    }

    fn sync(&mut self, actor: &crate::actor::Actor) {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
    }
}

impl crawler::CrawlerWorld for SimWalkerWorld<'_> {
    fn probe_ceiling(&mut self, actor: &crate::actor::Actor) -> bool {
        self.sync(actor);
        self.sim.m_ceil_own(self.st, self.layout, 18);
        self.sim.move_flags & 0x04 != 0
    }
}

impl walker::WalkerWorld for SimWalkerWorld<'_> {
    fn probe_left_wall(&mut self, actor: &crate::actor::Actor) -> bool {
        self.sync(actor);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall(&mut self, actor: &crate::actor::Actor) -> bool {
        self.sync(actor);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn probe_ground(&mut self, actor: &crate::actor::Actor) -> bool {
        self.sync(actor);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn step_fall(&mut self, actor: &mut crate::actor::Actor) -> u8 {
        falling_dropper::step_fall_speed(actor)
    }
}

impl ObjectSim {
    pub(super) fn handler_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = walker::WalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        let effect = walker::update(
            &mut actor,
            context,
            &mut SimWalkerWorld::new(self, st, layout),
        );
        if let walker::WalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    /// The `$5821E` walk legs over a slot whose frame state may sit above the
    /// leg number (the hopper's legs are states 2..=5).
    pub(super) fn walker_state(&mut self, st: &mut Staging, layout: &LevelLayout, leg: u8) {
        let leg = match leg {
            1 => walker::WalkLeg::WalkLeft,
            2 => walker::WalkLeg::WalkRight,
            3 => walker::WalkLeg::FallLeft,
            4 => walker::WalkLeg::FallRight,
            _ => return,
        };
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        walker::step_walk(&mut actor, leg, &mut SimWalkerWorld::new(self, st, layout));
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_fallwalker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.fallwalker_dispatch(st, layout, crate::actor::ContactKind::from_raw(0x56));
    }

    pub(super) fn handler_fallwalker52(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.fallwalker_dispatch(st, layout, crate::actor::ContactKind::from_raw(0x52));
    }

    pub(super) fn fallwalker_dispatch(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        contact_kind: crate::actor::ContactKind,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = fallwalker::FallWalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
            contact_kind,
        };
        let mut world = super::terrain_adapter::TerrainWorld::new(self, st, layout);
        let effect = fallwalker::update(&mut actor, context, &mut world);
        if let fallwalker::FallWalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_stun(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let effect = reactions::update_stun(
            &mut actor,
            frame,
            &mut SimWalkerWorld::new(self, st, layout),
        );
        st.b = actor.to_raw_slot();
        if effect == reactions::StunEffect::Revive {
            self.finish_sprite_swap(st, tables);
        }
    }

    pub(super) fn handler_vertical_pacer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = vertical_pacer::VerticalPacerContext {
            frame: self.frame_tick(),
        };
        let _ = vertical_pacer::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_perched_flyer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = perched_flyer::PerchedFlyerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        let effect = perched_flyer::update(&mut actor, context);
        if let perched_flyer::PerchedFlyerEffect::Startled { sfx, facing } = effect {
            self.sfx1 = sfx;
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_cannon_ant(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = cannon_ant::CannonAntContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
            mario_ref_y: self.mario_ref_y,
        };
        let effect = cannon_ant::update(&mut actor, context);
        st.b = actor.to_raw_slot();
        match effect {
            cannon_ant::CannonAntEffect::Initialized { facing } => {
                self.facing = facing;
            }
            cannon_ant::CannonAntEffect::FireProjectile {
                anim,
                sprite,
                attr,
                sfx,
            } => {
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = attr;
                self.spawn_dynamic_5b36d(st);
                self.sfx1 = sfx;
            }
            _ => {}
        }
    }

    pub(super) fn respawn_object_5b2bf(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if self.reset_record_origin(st, tables).is_err() {
            self.note_unported(0x5B2BF);
        }
    }

    pub(super) fn handler_wave_flyer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = wave_flyer::WaveFlyerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        let effect = wave_flyer::update(&mut actor, context);
        if let wave_flyer::WaveFlyerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_carrier46(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        struct CarrierSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> carrier::CarrierWorld for CarrierSimWorld<'a> {
            fn step_fall(&mut self, actor: &mut crate::actor::Actor) -> u8 {
                falling_dropper::step_fall_speed(actor)
            }
            fn probe_ground(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_ground_own(self.st, self.layout);
                self.sim.move_flags & 0x08 != 0
            }
        }
        let mut world = CarrierSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = carrier::update(&mut actor, &mut world);
        st.b = actor.to_raw_slot();
        match effect {
            carrier::CarrierEffect::Falling { carry_dy } => {
                self.carry_mario(st, carry_dy);
            }
            carrier::CarrierEffect::Despawn { carry_dy } => {
                self.carry_mario(st, carry_dy);
                self.remove_object(st, tables);
            }
            _ => {}
        }
    }

    pub(super) fn carry_mario(&mut self, st: &mut Staging, b: u8) {
        if st.b[B_FLAGS] & 0x08 == 0 {
            return;
        }
        let y = self.mario_y_adjust.unwrap_or(self.mario.y);
        self.mario_y_adjust = Some(y.wrapping_add(b as u16));
    }

    pub(super) fn carry_mario_x(&mut self, st: &mut Staging, dx: i16) {
        if st.b[B_FLAGS] & 0x08 == 0 {
            return;
        }
        self.carry_mario_dx(dx);
    }

    pub(super) fn handler_horizontal_platform(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = horizontal_platform::HorizontalPlatformContext {
            mario_ref_x: self.mario_ref_x,
        };
        let effect = horizontal_platform::update(&mut actor, context);
        match effect {
            horizontal_platform::HorizontalPlatformEffect::Initialized { facing } => {
                self.facing = facing;
            }
            horizontal_platform::HorizontalPlatformEffect::Moved { carry_dx, .. }
            | horizontal_platform::HorizontalPlatformEffect::Turned { carry_dx, .. } => {
                if let Some(dx) = carry_dx {
                    self.carry_mario_x(st, dx);
                }
            }
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_winged_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = winged_walker::WingedWalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        struct WingedWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> winged_walker::WingedWalkerWorld for WingedWalkerSimWorld<'a> {
            fn probe_left_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_left_wall(self.st, self.layout);
                self.sim.move_flags & 0x02 != 0
            }
            fn probe_right_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_right_wall(self.st, self.layout);
                self.sim.move_flags & 0x01 != 0
            }
            fn probe_ground(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_ground_own(self.st, self.layout);
                self.sim.move_flags & 0x08 != 0
            }
            fn probe_swoop_left_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_left_wall_high(self.st, self.layout);
                self.sim.move_flags & 0x02 != 0
            }
            fn probe_swoop_right_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_right_wall_high(self.st, self.layout);
                self.sim.move_flags & 0x01 != 0
            }
        }
        let mut world = WingedWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = winged_walker::update(&mut actor, context, &mut world);
        match effect {
            winged_walker::WingedWalkerEffect::Initialized { facing }
            | winged_walker::WingedWalkerEffect::Landed { facing, .. } => {
                self.facing = facing;
            }
            _ => {}
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_edge_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = edge_walker::EdgeWalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        struct EdgeWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> edge_walker::EdgeWalkerWorld for EdgeWalkerSimWorld<'a> {
            fn probe_left_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_left_wall(self.st, self.layout);
                self.sim.move_flags & 0x02 != 0
            }
            fn probe_right_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_right_wall(self.st, self.layout);
                self.sim.move_flags & 0x01 != 0
            }
            fn probe_ground(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_ground_own(self.st, self.layout);
                self.sim.move_flags & 0x08 != 0
            }
        }
        let mut world = EdgeWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = edge_walker::update(&mut actor, context, &mut world);
        if let edge_walker::EdgeWalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_left_marcher(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = left_marcher::LeftMarcherContext {
            frame: self.frame_tick(),
        };
        let _effect = left_marcher::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_falling_dropper(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = falling_dropper::FallingDropperContext {
            mario_ref_x: self.mario_ref_x,
        };
        let effect = falling_dropper::update(&mut actor, context);
        if let falling_dropper::FallingDropperEffect::TriggerDrop { sfx } = effect {
            self.sfx1 = sfx;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_dashing_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = dashing_walker::DashingWalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        struct DashingWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> dashing_walker::DashingWalkerWorld for DashingWalkerSimWorld<'a> {
            fn probe_left_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_left_wall_wide(self.st, self.layout);
                self.sim.move_flags & 0x02 != 0
            }
            fn probe_right_wall(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_right_wall_wide(self.st, self.layout);
                self.sim.move_flags & 0x01 != 0
            }
        }
        let mut world = DashingWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = dashing_walker::update(&mut actor, context, &mut world);
        if let dashing_walker::DashingWalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    /// `$58712`: the hopper.
    pub(super) fn handler_hopper(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = tables;
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let revision = self.revision;
        let effect = hopper::update(&mut actor, &mut SimWalkerWorld::new(self, st, layout), revision);
        st.b = actor.to_raw_slot();
        if effect == hopper::Effect::Parked {
            self.done = 1;
        }
    }

    pub(super) fn handler_bobber(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
        direction: pipe_bobber::EmergenceDirection,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = pipe_bobber::PipeBobberContext {
            frame: self.frame_tick(),
            mario_travelling: self.mario.travelling,
            mario_ref_x: self.mario_ref_x,
        };
        let effect = pipe_bobber::update(&mut actor, context, direction);
        st.b = actor.to_raw_slot();
        if effect == pipe_bobber::PipeBobberEffect::Despawn {
            self.remove_object(st, tables);
        }
    }

    pub(super) fn handler_fire_piranha(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = fire_piranha::FirePiranhaContext {
            frame: self.frame_tick(),
            mario_travelling: self.mario.travelling,
            mario_ref_x: self.mario_ref_x,
            mario_ref_y: self.mario_ref_y,
        };
        let effect = fire_piranha::update(&mut actor, context);
        st.b = actor.to_raw_slot();
        match effect {
            fire_piranha::FirePiranhaEffect::Despawn => {
                self.remove_object(st, tables);
            }
            fire_piranha::FirePiranhaEffect::SpawnProjectile { flip } => {
                self.spawn_anim = 14;
                self.spawn_sprite = 22;
                self.spawn_attr = flip.bits();
                self.spawn_dynamic_5b36d(st);
            }
            _ => {}
        }
    }

    pub(super) fn handler_piranha_fireball(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = piranha_fireball::FireballContext {
            frame: self.frame_tick(),
        };
        let _effect = piranha_fireball::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_boo(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = boo::BooContext {
            frame: self.frame_tick(),
            mario_dir_right: self.mario.dir_right,
            mario_ref_x: self.mario_ref_x,
            mario_ref_y: self.mario_ref_y,
        };
        if context.frame.every(4) {
            let facing_x = boo::object_facing_to_mario(context.mario_ref_x, actor.position.x);
            let mario_dir = if context.mario_dir_right { 1 } else { 255 };
            self.facing = if facing_x == mario_dir && matches!(actor.state, 1 | 2) {
                boo::object_facing_to_mario(context.mario_ref_y, actor.position.y)
            } else {
                facing_x
            };
        }
        let result = boo::update(&mut actor, context);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x59545);
        }
    }

    /// `$5862E` (anim 8).
    pub(super) fn handler_crawler(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.run_crawler(st, layout, true);
    }

    /// `$58642`: the shared crawl body the other crawlers enter.
    pub(super) fn crawler_body(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.run_crawler(st, layout, false);
    }

    /// `entry` runs anim 8's own blink and facing before the crawl.
    fn run_crawler(&mut self, st: &mut Staging, layout: &LevelLayout, entry: bool) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let mario_ref_x = self.mario_ref_x;
        let world = &mut SimWalkerWorld::new(self, st, layout);
        let faced = if entry {
            crawler::update(&mut actor, frame, mario_ref_x, world)
        } else {
            crawler::crawl(&mut actor, frame, mario_ref_x, world)
        };
        st.b = actor.to_raw_slot();
        if let Some(side) = faced {
            self.facing = match side {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => 0xFF,
                std::cmp::Ordering::Equal => 0,
            };
        }
    }

    pub(super) fn handler_death(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = layout;
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let result = reactions::update_defeat(&mut actor);
        st.b = actor.to_raw_slot();
        match result {
            Ok(reactions::DefeatEffect::None) => {}
            Ok(reactions::DefeatEffect::Remove) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
                self.count_down_boss_defeated();
            }
            Err(_) => self.note_unported(0x581CE),
        }
    }
}

impl ObjectSim {
    pub(super) fn hit_reaction(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let mario_ref_x = self.mario_ref_x;
        let effect = reactions::update_knockback(
            &mut actor,
            mario_ref_x,
            &mut SimWalkerWorld::new(self, st, layout),
        );
        st.b = actor.to_raw_slot();
        match effect {
            reactions::KnockbackEffect::None => {}
            reactions::KnockbackEffect::Faced(side) => {
                self.facing = match side {
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Less => 0xFF,
                    std::cmp::Ordering::Equal => 0,
                };
            }
            // `$5819B`: the landing re-derives the sprite but leaves the
            // slot's handled flag alone.
            reactions::KnockbackEffect::Landed { resume } => {
                self.derive_sprite(st, tables);
                if let Some(contact_kind) = resume {
                    let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
                    reactions::resume_walk(&mut actor, contact_kind);
                    st.b = actor.to_raw_slot();
                }
            }
        }
    }
}
