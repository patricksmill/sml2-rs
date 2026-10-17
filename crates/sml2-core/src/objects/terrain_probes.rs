//! Shared native terrain geometry, with the source's final probe projection.
use crate::{actor::Position, level::LevelLayout};

#[derive(Clone, Copy)]
enum ProbeContact {
    Wall { right: bool, blocked: bool },
    Ground { grounded: bool },
}

pub(super) struct TerrainProbes<'a> {
    layout: &'a LevelLayout,
    position: Position,
    contact: Option<ProbeContact>,
}

impl<'a> TerrainProbes<'a> {
    pub(super) fn new(layout: &'a LevelLayout, initial: Position) -> Self {
        Self {
            layout,
            position: initial,
            contact: None,
        }
    }

    pub(super) fn position(&self) -> Position {
        self.position
    }

    fn tile(&self) -> u8 {
        self.layout
            .get(
                usize::from((self.position.x >> 4) & 255),
                usize::from((self.position.y >> 4) & 255),
            )
            .unwrap_or(0)
    }

    /// Source $843C/$84EA use reach 7; $8462/$8510 use reach 11.
    pub(super) fn wall_blocked(&mut self, actor: Position, right: bool, reach: u16) -> bool {
        self.position = Position {
            x: if right {
                actor.x.wrapping_add(reach)
            } else {
                actor.x.wrapping_sub(reach)
            },
            y: actor.y.wrapping_sub(11),
        };
        let blocked = if self.tile() < 56 {
            true
        } else {
            self.position.y = actor.y.wrapping_sub(5);
            self.tile() < 56
        };
        self.contact = Some(ProbeContact::Wall { right, blocked });
        blocked
    }

    /// Source $8358 accepts low Y alignment 0..3, and tiles below 72.
    pub(super) fn grounded(&mut self, actor: Position) -> bool {
        // Even the alignment early return writes low Y, retaining old high Y/X.
        self.position.y = (self.position.y & 0xff00) | (actor.y & 255);
        let grounded = if actor.y & 12 != 0 {
            false
        } else {
            self.position = actor;
            self.tile() < 72
        };
        self.contact = Some(ProbeContact::Ground { grounded });
        grounded
    }

    /// Compatibility flags are projected only at the legacy world boundary.
    pub(super) fn legacy_move_flags(&self) -> Option<u8> {
        self.contact.map(|contact| match contact {
            ProbeContact::Wall {
                right: true,
                blocked,
            } => 0x10 | u8::from(blocked),
            ProbeContact::Wall {
                right: false,
                blocked,
            } => 0x20 | (u8::from(blocked) << 1),
            ProbeContact::Ground { grounded } => 0x80 | (u8::from(grounded) << 3),
        })
    }
}

#[cfg(test)]
#[path = "terrain_probes_tests.rs"]
mod tests;
