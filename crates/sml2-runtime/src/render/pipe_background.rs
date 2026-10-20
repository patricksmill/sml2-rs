//! `$2323` changes room pages, but `$2359`/`$23E4` replace only incoming
//! columns in the BG tilemap. Retain the source columns until they leave view.
use sml2_core::{camera::CameraSim, scene::SceneStep};

#[derive(Debug, Clone, Copy)]
pub struct PipeBackground {
    offset: (i32, i32),
    boundary: i32,
    right: bool,
}

impl PipeBackground {
    pub fn update(active: &mut Option<Self>, before: (u16, u16), cam: &CameraSim, step: SceneStep) {
        if cam.transition == 0 {
            *active = None;
        } else if matches!(step, SceneStep::Ran(op) if op >> 4 == 4)
            && matches!(cam.transition, 1 | 2)
        {
            // Right streams cam+80, +96, +112; left streams cam-96,
            // -112, -128. Each write replaces a whole 16-pixel column.
            let right = cam.transition == 1;
            let boundary = if right {
                (cam.x as i32 + 80).div_euclid(16) * 16
            } else {
                (cam.x as i32 - 96).div_euclid(16) * 16 + 16
            };
            *active = Some(Self {
                offset: (
                    before.0 as i32 - cam.x as i32,
                    before.1 as i32 - cam.y as i32,
                ),
                boundary,
                right,
            });
        }
    }

    pub(super) fn source_offset(self) -> (i32, i32) {
        self.offset
    }

    pub(super) fn retains_source(self, x: i32) -> bool {
        if self.right {
            x < self.boundary
        } else {
            x >= self.boundary
        }
    }

    pub(super) fn sample(self, x: i32, y: i32) -> (i32, i32) {
        if self.retains_source(x) {
            (x + self.offset.0, y + self.offset.1)
        } else {
            (x, y)
        }
    }
}
