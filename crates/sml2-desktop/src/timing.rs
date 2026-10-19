use std::time::Duration;

/// Nominal 60 Hz, independent of redraw cadence; pauses discard accumulated time.
#[derive(Default)]
pub struct TickBudget {
    phase: u128,
}
impl TickBudget {
    pub fn reset(&mut self) {
        self.phase = 0;
    }
    pub fn advance(&mut self, elapsed: Duration, running: bool) -> usize {
        if !running {
            self.reset();
            return 0;
        }
        self.phase += elapsed.as_nanos() * 60;
        let due = self.phase / 1_000_000_000;
        self.phase %= 1_000_000_000;
        // Drop excess after a stall, rather than performing an unbounded burst.
        due.min(8) as usize
    }
}

/// Pixel-aligned destination, computed in physical pixels, not UI points.
pub fn game_rectangle(available: [u32; 2], zoom: crate::settings::Zoom) -> ([u32; 2], [u32; 2]) {
    let fit = (available[0] / 160).min(available[1] / 144);
    if fit == 0 {
        return ([0, 0], [0, 0]);
    }
    let scale = match zoom {
        crate::settings::Zoom::Auto => fit,
        crate::settings::Zoom::Integer(n) => u32::from(n).min(fit).max(1),
    };
    let size = [160 * scale, 144 * scale];
    (
        [(available[0] - size[0]) / 2, (available[1] - size[1]) / 2],
        size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refresh_rate_does_not_change_ticks_and_pause_cannot_build_up_debt() {
        for refresh in [60u64, 90, 120, 144] {
            let mut budget = TickBudget::default();
            let mut ticks = 0;
            for frame in 0..refresh {
                let a = frame * 1_000_000_000 / refresh;
                let b = (frame + 1) * 1_000_000_000 / refresh;
                ticks += budget.advance(Duration::from_nanos(b - a), true);
            }
            assert_eq!(ticks, 60);
            assert_eq!(budget.advance(Duration::from_secs(10), false), 0);
            assert_eq!(budget.advance(Duration::ZERO, true), 0);
            assert_eq!(budget.advance(Duration::from_secs(10), true), 8);
        }
    }
    #[test]
    fn zoom_is_integer_centered_and_constrained_by_physical_window_pixels() {
        assert_eq!(
            game_rectangle([1000, 700], crate::settings::Zoom::Auto),
            ([180, 62], [640, 576])
        );
        assert_eq!(
            game_rectangle([1000, 700], crate::settings::Zoom::Integer(2)),
            ([340, 206], [320, 288])
        );
        assert_eq!(
            game_rectangle([0, 0], crate::settings::Zoom::Auto),
            ([0, 0], [0, 0])
        );
    }
}
