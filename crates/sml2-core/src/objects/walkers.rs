//! Object walkers routines.

use super::*;

impl ObjectSim {
    pub(super) fn handler_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = walker::WalkerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
        };
        struct WalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> walker::WalkerWorld for WalkerSimWorld<'a> {
            fn probe_left_wall(&mut self, _actor: &crate::actor::Actor) -> bool {
                self.sim.m_left_wall(self.st, self.layout);
                self.sim.move_flags & 0x02 != 0
            }
            fn probe_right_wall(&mut self, _actor: &crate::actor::Actor) -> bool {
                self.sim.m_right_wall(self.st, self.layout);
                self.sim.move_flags & 0x01 != 0
            }
            fn probe_ground(&mut self, actor: &crate::actor::Actor) -> bool {
                set_x(&mut self.st.b, actor.position.x);
                set_y(&mut self.st.b, actor.position.y);
                self.sim.m_ground_own(self.st, self.layout);
                self.sim.move_flags & 0x08 != 0
            }
            fn step_fall(&mut self, actor: &mut crate::actor::Actor) -> u8 {
                let dy = self.sim.fall_step(self.st);
                actor.motion_phase = self.st.b[B_PHASE];
                dy
            }
        }
        let mut world = WalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = walker::update(&mut actor, context, &mut world);
        if let walker::WalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn walker_state(&mut self, st: &mut Staging, layout: &LevelLayout, state: u8) {
        match state {
            // $4226: walk left; wall on the left → turn; no ground → fall.
            1 => {
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_sub(st, 1);
                self.walk_ground_check(st, layout);
            }
            // $4253: walk right; wall on the right → turn.
            2 => {
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_add(st, 1);
                self.walk_ground_check(st, layout);
            }
            // $4271: falling left.
            3 => {
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_sub(st, 1);
                self.fall_step_and_land(st, layout);
            }
            // $4282: falling right.
            4 => {
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_add(st, 1);
                self.fall_step_and_land(st, layout);
            }
            _ => {}
        }
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
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
            contact_kind,
        };
        struct FallWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> fallwalker::FallWalkerWorld for FallWalkerSimWorld<'a> {
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
            fn step_fall(&mut self, actor: &mut crate::actor::Actor) -> u8 {
                let dy = self.sim.fall_step(self.st);
                actor.motion_phase = self.st.b[B_PHASE];
                dy
            }
        }
        let mut world = FallWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
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
        match st.b[B_FSTATE] {
            // $58364: count down; fall when airborne.
            1 => {
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] == 0 {
                    st.b[B_FSTATE] = 2;
                    return;
                }
                self.m_ground_own(st, layout);
                if self.move_flags & 0x08 != 0 {
                    return;
                }
                let b = self.fall_step(st);
                self.mv_y_add(st, b);
            }
            // $58380: spin, then revive to anim 1 ($3AA0 sprite re-derive).
            2 => {
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] ^= 0x0B;
                st.b[B_T28] = st.b[B_T28].wrapping_add(1);
                if st.b[B_T28] != 8 {
                    return;
                }
                st.b[B_DEF] = 1;
                self.finish_sprite_swap(st, tables);
            }
            // fstate 0: init.
            _ => {
                st.b[B_TYPE] = 0x06;
                st.b[B_FSTATE] = 1;
                st.b[B_T28] = 192;
            }
        }
    }

    pub(super) fn handler_groundwalker88(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = groundwalker::GroundWalkerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
        };
        struct GroundWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> groundwalker::GroundWalkerWorld for GroundWalkerSimWorld<'a> {
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
        }
        let mut world = GroundWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = groundwalker::update(&mut actor, context, &mut world);
        if let groundwalker::GroundWalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_paced_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = paced_walker::PacedWalkerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
        };
        struct PacedWalkerSimWorld<'a> {
            sim: &'a mut ObjectSim,
            st: &'a mut Staging,
            layout: &'a LevelLayout,
        }
        impl<'a> paced_walker::PacedWalkerWorld for PacedWalkerSimWorld<'a> {
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
        }
        let mut world = PacedWalkerSimWorld {
            sim: self,
            st,
            layout,
        };
        let effect = paced_walker::update(&mut actor, context, &mut world);
        if let paced_walker::PacedWalkerEffect::Initialized { facing } = effect {
            self.facing = facing;
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_vertical_pacer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = vertical_pacer::VerticalPacerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
        };
        let _ = vertical_pacer::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_perched_flyer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = perched_flyer::PerchedFlyerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
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
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
            mario_ref_y: self.mario_ref_y,
        };
        let effect = cannon_ant::update(&mut actor, context);
        st.b = actor.to_raw_slot();
        match effect {
            cannon_ant::CannonAntEffect::Initialized { facing } => {
                self.facing = facing;
            }
            cannon_ant::CannonAntEffect::FireProjectile { anim, sprite, attr, sfx } => {
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
        if let Some(ri) = self.record_index(st) {
            let rec = &self.records[ri];
            set_x(&mut st.b, rec.x);
            set_y(&mut st.b, rec.y);
            st.b[B_MARKER] = rec.mode_flag;
            self.derive_sprite(st, tables);
            st.b[B_STATE] = 1;
            self.active = self.active.wrapping_sub(1);
            self.standby = self.standby.wrapping_add(1);
        } else {
            st.b[B_STATE] = 1;
        }
    }

    pub(super) fn handler_charging_enemy(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = charging_enemy::ChargingEnemyContext {
            mario_ref_x: self.mario_ref_x,
        };
        let effect = charging_enemy::update(&mut actor, context);
        match effect {
            charging_enemy::ChargingEnemyEffect::FacingSelected { facing } => {
                self.facing = facing;
            }
            charging_enemy::ChargingEnemyEffect::ChargeTriggered { sfx, .. } => {
                self.sfx1 = sfx;
            }
            charging_enemy::ChargingEnemyEffect::OffscreenReset => {
                self.respawn_object_5b2bf(st, tables);
                return;
            }
            _ => {}
        }
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_wave_flyer(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = wave_flyer::WaveFlyerContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
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
                let dy = self.sim.fall_step(self.st);
                actor.motion_phase = self.st.b[B_PHASE];
                dy
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

    pub(super) fn handler_hopper(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = tables;
        match st.b[B_FSTATE] {
            1 => {
                // $58734: rise 1 px per call; at 8 → state 3, clear +12.
                self.mv_y_sub(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_add(1);
                if st.b[B_T24] != 8 {
                    return;
                }
                st.b[B_FSTATE] = 3;
                st.b[B_DEFEAT] = 0;
            }
            2 => self.walker_state(st, layout, 1),
            3 => self.walker_state(st, layout, 2),
            4 => self.walker_state(st, layout, 3),
            5 => self.walker_state(st, layout, 4),
            _ => {
                // state 0: item setup.
                st.b[B_TYPE] = 0x80;
                st.b[B_T24] = 0;
                st.b[B_DEFEAT] = 0x80;
                self.done = 1;
            }
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
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_travelling: self.mario.travelling,
            mario_ref_x: self.mario_ref_x,
        };
        let effect = pipe_bobber::update(&mut actor, context, direction);
        st.b = actor.to_raw_slot();
        if effect == pipe_bobber::PipeBobberEffect::Despawn {
            self.remove_object(st, tables);
        }
    }

    pub(super) fn handler_fire_piranha(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
    ) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = fire_piranha::FirePiranhaContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
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
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
        };
        let _effect = piranha_fireball::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_boo(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = boo::BooContext {
            frame: crate::actor::FrameTick(u64::from(self.fff0)),
            mario_dir_right: self.mario.dir_right,
            mario_ref_x: self.mario_ref_x,
            mario_ref_y: self.mario_ref_y,
        };
        let _effect = boo::update(&mut actor, context);
        st.b = actor.to_raw_slot();
    }

    pub(super) fn handler_crawler(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.sprite_toggle1(st);
        match st.b[B_FSTATE] {
            0 => {
                self.face_mario(st);
                st.b[B_FLASH] = 16;
                st.b[B_TYPE] = 0x56;
            }
            1 | 3 => {
                // $5864E
                if self.fff0 & 1 == 0 {
                    self.mv_x_sub(st, 1);
                    self.m_left_wall(st, layout);
                    if self.move_flags & 0x02 != 0 {
                        st.b[B_FSTATE] = 2;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            2 | 4 => {
                // $5866D
                if self.fff0 & 1 == 0 {
                    self.mv_x_add(st, 1);
                    self.m_right_wall(st, layout);
                    if self.move_flags & 0x01 != 0 {
                        st.b[B_FSTATE] = 1;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            _ => {}
        }
    }

    pub(super) fn crawler_vertical(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if st.b[B_T24] == 1 {
            // $586AD — falling.
            let b = self.fall_step(st);
            self.mv_y_add(st, b);
            self.m_ground_own(st, layout);
            if self.move_flags & 0x08 == 0 {
                return;
            }
            st.b[B_T24] = 0;
            st.b[B_T25] = st.b[B_T25].wrapping_add(1);
            if st.b[B_T25] == 4 {
                // $586D6: restarted cycle, face Mario.
                st.b[B_PHASE] = 0;
                st.b[B_T24] = 0;
                st.b[B_T25] = 0;
                self.face_mario(st);
                return;
            }
            st.b[B_PHASE] = 10;
            if st.b[B_FSTATE] >= 3 {
                return;
            }
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
            return;
        }
        // $5868C — hovering.
        let b = self.hover_step(st);
        if st.b[B_PHASE] == 21 {
            st.b[B_PHASE] = 0;
            st.b[B_T24] = 1;
            return;
        }
        self.mv_y_sub(st, b);
        self.m_ceil_own(st, layout, 18);
        if self.move_flags & 0x04 == 0 {
            return;
        }
        st.b[B_PHASE] = 0;
        st.b[B_T24] = 1;
    }

    pub(super) fn handler_death(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = layout;
        match st.b[B_FSTATE] {
            // $581CE: rise on 2x the $74AB curve.
            1 => {
                let (b, c) = self.step_picker(st, &DEATH_RISE_74AB, 8);
                self.mv_y_sub(st, b.wrapping_mul(2));
                if st.b[B_PHASE] != c {
                    return;
                }
                st.b[B_FSTATE] = 2;
                st.b[B_PHASE] = 0;
            }
            // $581E2: fall on 2x the $5B451 curve; off the bottom of the
            // screen -> `$5B2D7` with remove-mode 2.
            2 => {
                let b = self.fall_step(st);
                self.mv_y_add(st, b.wrapping_mul(2));
                if st.b[B_SCRY] < 0xA0 {
                    return;
                }
                self.remove_mode = 2;
                self.remove_object(st, tables);
                // $581E2's tail also counts `sBossDefeated` down; the port
                // models that byte as a bool (the boss item), so there is
                // nothing to decrement here yet.
            }
            // $581B9: the hop + the flip.
            _ => {
                self.mv_y_sub(st, 20);
                st.b[B_FLIP] ^= 0x40;
                st.b[B_PHASE] = 0;
                st.b[B_TYPE] = 0;
                st.b[B_FSTATE] = 1;
            }
        }
    }

    pub(super) fn handler_kicked(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if st.b[B_TYPE] & 0x40 != 0 {
            self.ko_overlap(st);
        }
        self.spin_step(st); // $5B607 - byte-identical to $5B545's wobble
        match st.b[B_FSTATE] {
            0 => {
                // $5B3C6 + type $5F.
                self.face_mario(st);
                st.b[B_TYPE] = 0x5F;
            }
            // $83BC / $83E6: walk. The probes are the *low* pair ($3BA4/$3C08),
            // not the anim-9 walker's Y-11 ones.
            1 => {
                self.m_right_wall_low(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1); // $583DB
                    return;
                }
                self.mv_x_add(st, 3);
                self.walk_ground_check(st, layout); // $583CB: no ground -> state += 2
            }
            2 => {
                self.m_left_wall_low(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1); // $583F7
                    return;
                }
                self.mv_x_sub(st, 3);
                self.walk_ground_check(st, layout);
            }
            // $8402 / $8433: the falls.
            3 => {
                self.m_right_wall_low(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    return;
                }
                self.mv_x_add(st, 3);
                self.kick_fall(st, layout);
            }
            4 => {
                self.m_left_wall_low(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    return;
                }
                self.mv_x_sub(st, 3);
                self.kick_fall(st, layout);
            }
            _ => {}
        }
    }

    pub(super) fn kick_fall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 != 0 {
            st.b[B_PHASE] = 0;
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(2);
            st.b[B_YLO] &= 0xF0;
            return;
        }
        let b = self.fall_step(st);
        self.mv_y_add(st, b.wrapping_add(1));
    }
}

impl ObjectSim {
    pub(super) fn hit_reaction(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        if st.b[B_FSTATE] == 0 {
            self.face_mario(st);
            st.b[B_FLIP] ^= 0x20;
            return;
        }
        match st.b[B_FSTATE].wrapping_sub(1) {
            // $411A: knocked right — X+1, wall probe; bounce up; the state
            // advances +2 every 15 phases.
            0 => {
                self.mv_x_add(st, 1);
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                }
                let b = self.bounce_step(st);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
                }
            }
            // $4142: knocked left.
            1 => {
                self.mv_x_sub(st, 1);
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                }
                let b = self.bounce_step(st);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
                }
            }
            // $4158: airborne right — fall; land (revive or remove).
            2 => {
                self.mv_x_add(st, 1);
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                }
                self.land_or_remove(st, layout, tables);
            }
            // $4185: airborne left.
            _ => {
                self.mv_x_sub(st, 1);
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                }
                self.land_or_remove(st, layout, tables);
            }
        }
    }

    pub(super) fn land_or_remove(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let b = self.fall_step(st);
        self.mv_y_add(st, b);
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 == 0 {
            return;
        }
        if st.b[B_HITREACT] == 2 {
            // $5819B: revive — sprite re-derive, then the frame state is set
            // from the flip byte ($3CB1: 1/2) and advanced.
            let saved_type = st.b[B_TYPE];
            let d = self.done;
            self.finish_sprite_swap(st, tables);
            self.done = d;
            st.b[B_TYPE] = saved_type;
            st.b[B_FSTATE] = if st.b[B_FLIP] & 0x20 != 0 { 3 } else { 2 };
        } else {
            let d = self.done;
            self.finish_sprite_swap(st, tables);
            self.done = d;
        }
    }

    pub(super) fn walk_ground_check(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 == 0 {
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
        }
    }

    pub(super) fn fall_step_and_land(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let b = self.fall_step(st);
        self.mv_y_add(st, b);
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 != 0 {
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(2);
        }
    }
}
