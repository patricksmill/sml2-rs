//! Second pig's intro drop and repeating hops, bank 2 `$9D83..$9E06`.
use crate::actor::{Actor, RiseProfile, SpriteFlipFlags};

/// Returns the sound requested on a wall turn or completed hop.
pub(super) fn update(actor: &mut Actor, fall: &[u8]) -> Result<Option<u8>, ()> {
    if actor.state == 3 {
        actor.screen.move_x(-1);
        actor.timer = 8;
        // `$9D83` uses the shared intro drop `$9CDF`, which lands facing left.
        if actor.motion_leg == 0 {
            let dy = actor.rise_step(RiseProfile::Low).map_err(|_| ())?;
            actor.screen.move_y(-i16::from(dy));
            if actor.motion_phase == 15 {
                actor.motion_phase = 0;
                actor.motion_leg = 1;
            }
        } else {
            let dy = actor.motion_step(fall, 21).map_err(|_| ())?;
            actor.screen.move_y(i16::from(dy));
            if actor.screen.y >= 144 {
                actor.screen.y = 144;
                actor.motion_phase = 0;
                actor.motion_leg = 0;
                actor.state = 4;
                actor.sprite_flip = SpriteFlipFlags::from_bits(0x20);
            }
        }
        return Ok(None);
    }
    if !matches!(actor.state, 4 | 5) {
        return Err(());
    }
    if actor.timer != 0 {
        actor.sprite = 40;
        actor.timer -= 1;
        if actor.timer != 0 {
            return Ok(None);
        }
        actor.sprite = 32;
    }
    let speed = 4u8.wrapping_sub(actor.activity >> 1);
    let left = actor.state == 4;
    actor.screen.move_x(if left {
        -i16::from(speed)
    } else {
        i16::from(speed)
    });
    if (left && actor.screen.x < 16) || (!left && actor.screen.x >= 160) {
        // `$9D3D` returns from this update before vertical motion.
        actor.state ^= 1;
        actor.sprite_flip.toggle_horizontal();
        return Ok(Some(actor.definition));
    }
    if actor.motion_leg == 0 {
        let dy = actor.rise_step(RiseProfile::Low).map_err(|_| ())?;
        actor.screen.move_y(-i16::from(dy));
    } else {
        let dy = actor.motion_step(fall, 15).map_err(|_| ())?;
        actor.screen.move_y(i16::from(dy));
    }
    if actor.motion_phase == 15 {
        actor.motion_phase = 0;
        actor.motion_leg ^= 1;
        if actor.motion_leg == 0 {
            actor.timer = 8;
            return Ok(Some(actor.definition));
        }
    }
    Ok(None)
}
