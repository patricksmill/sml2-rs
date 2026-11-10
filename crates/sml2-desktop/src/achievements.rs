//! Local achievements presentation: completion profile, unlock banners and
//! timestamp formatting. The runtime owns evaluation; this module never talks
//! to a network and never stores credentials.

use crate::settings::UnlockedAchievement;
use eframe::egui::{self, Align2, Color32, Context, FontId, Margin, Stroke, Vec2};
use sml2_core::achievements::{AchievementId, ALL_ACHIEVEMENTS};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long one banner stays on screen before the next queued unlock.
pub const TOAST_DURATION: Duration = Duration::from_secs(4);

/// Gold used by the banner border, matching the settings highlight.
const GOLD: Color32 = Color32::from_rgb(255, 220, 100);

/// Device-local completion figures. Only ids this build knows score points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementProfile {
    pub unlocked: usize,
    pub total: usize,
    pub score: u32,
    pub total_score: u32,
}

impl AchievementProfile {
    pub fn completion(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.unlocked as f32 / self.total as f32
        }
    }
}

pub fn profile(unlocked: &[UnlockedAchievement]) -> AchievementProfile {
    let mut profile = AchievementProfile {
        unlocked: 0,
        total: ALL_ACHIEVEMENTS.len(),
        score: 0,
        total_score: 0,
    };
    for id in ALL_ACHIEVEMENTS {
        let points = id.metadata().points;
        profile.total_score += points;
        if unlocked.iter().any(|entry| entry.id == id.id()) {
            profile.unlocked += 1;
            profile.score += points;
        }
    }
    profile
}

/// Unix seconds to `YYYY-MM-DD HH:MM UTC`. `0` means the time was not recorded.
pub fn format_unlocked_at(seconds: u64) -> String {
    if seconds == 0 {
        return "time not recorded".into();
    }
    let days = (seconds / 86_400) as i64;
    let remaining = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        remaining / 3600,
        remaining % 3600 / 60
    )
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementToast {
    pub id: AchievementId,
    pub title: &'static str,
    pub points: u32,
    /// When this banner started its four seconds (reset when promoted).
    pub started_at: Instant,
}

/// Unlock banners play one at a time, each for [`TOAST_DURATION`].
#[derive(Debug, Default)]
pub struct AchievementToasts {
    queue: VecDeque<AchievementToast>,
}

impl AchievementToasts {
    /// Queue a banner. A duplicate that is still queued is ignored.
    pub fn push(&mut self, id: AchievementId, now: Instant) -> bool {
        if self.queue.iter().any(|toast| toast.id == id) {
            return false;
        }
        let metadata = id.metadata();
        self.queue.push_back(AchievementToast {
            id,
            title: metadata.title,
            points: metadata.points,
            started_at: now,
        });
        true
    }

    /// Retire expired banners; the next one starts its full four seconds.
    pub fn update(&mut self, now: Instant) {
        loop {
            let Some(front) = self.queue.front() else {
                return;
            };
            if now.saturating_duration_since(front.started_at) < TOAST_DURATION {
                return;
            }
            self.queue.pop_front();
            if let Some(next) = self.queue.front_mut() {
                next.started_at = now;
            }
        }
    }

    pub fn active(&self) -> Option<&AchievementToast> {
        self.queue.front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Draw the current banner at top centre, gold-bordered, fading in the last
    /// fraction of a second.
    pub fn paint(&self, ctx: &Context, now: Instant) {
        let Some(toast) = self.active() else {
            return;
        };
        let elapsed = now.saturating_duration_since(toast.started_at);
        let left = TOAST_DURATION.saturating_sub(elapsed);
        let fade = (left.as_secs_f32() / 0.4).min(1.0);
        let alpha = (fade * 255.0) as u8;
        let fill = Color32::from_rgba_unmultiplied(24, 20, 8, (alpha as f32 * 0.92) as u8);
        let border = GOLD.gamma_multiply(fade);
        let ink = Color32::from_rgba_unmultiplied(255, 248, 225, alpha);
        let muted = Color32::from_rgba_unmultiplied(214, 196, 150, alpha);
        egui::Area::new(egui::Id::new("achievement-toast"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 12.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(fill)
                    .stroke(Stroke::new(2.0_f32, border))
                    .corner_radius(4)
                    .inner_margin(Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("ACHIEVEMENT UNLOCKED")
                                    .font(FontId::proportional(12.0))
                                    .color(muted),
                            );
                            ui.label(
                                egui::RichText::new(toast.title)
                                    .font(FontId::proportional(18.0))
                                    .strong()
                                    .color(ink),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} points · local achievement",
                                    toast.points
                                ))
                                .font(FontId::proportional(12.0))
                                .color(muted),
                            );
                        });
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_counts_known_unlocks_and_sums_their_points() {
        use AchievementId::{BranchingOut, WhereNoPlumberGoneBefore};
        let unlocked = vec![
            UnlockedAchievement {
                id: BranchingOut.id(),
                unlocked_at: 0,
            },
            UnlockedAchievement {
                id: WhereNoPlumberGoneBefore.id(),
                unlocked_at: 0,
            },
            // A future id this build does not know is ignored.
            UnlockedAchievement {
                id: 4_000_000,
                unlocked_at: 0,
            },
        ];
        let profile = profile(&unlocked);
        assert_eq!(profile.unlocked, 2);
        assert_eq!(profile.total, 28);
        assert_eq!(profile.score, 20);
        assert_eq!(profile.total_score, 302, "the set is worth 302 points");
    }

    #[test]
    fn timestamps_convert_to_utc_without_a_dependency() {
        assert_eq!(format_unlocked_at(0), "time not recorded");
        assert_eq!(format_unlocked_at(1_700_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(format_unlocked_at(951_782_400), "2000-02-29 00:00 UTC");
    }

    #[test]
    fn banners_play_one_at_a_time_for_four_seconds() {
        let now = Instant::now();
        let mut toasts = AchievementToasts::default();
        assert!(toasts.push(ALL_ACHIEVEMENTS[0], now));
        assert!(
            !toasts.push(ALL_ACHIEVEMENTS[0], now),
            "duplicate queued id"
        );
        assert!(toasts.push(ALL_ACHIEVEMENTS[1], now));
        assert_eq!(toasts.len(), 2);
        assert_eq!(toasts.active().unwrap().id, ALL_ACHIEVEMENTS[0]);
        toasts.update(now + TOAST_DURATION - Duration::from_millis(1));
        assert_eq!(toasts.active().unwrap().id, ALL_ACHIEVEMENTS[0]);
        toasts.update(now + TOAST_DURATION);
        assert_eq!(toasts.active().unwrap().id, ALL_ACHIEVEMENTS[1]);
        assert_eq!(
            toasts.active().unwrap().started_at,
            now + TOAST_DURATION,
            "the promoted banner gets its own four seconds"
        );
        toasts.update(now + TOAST_DURATION * 2);
        assert!(toasts.is_empty());
    }
}
