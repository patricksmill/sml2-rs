use super::*;
use crate::timing::TickBudget;
use std::time::Duration;

#[test]
fn catch_up_ticks_each_render_audio_at_the_live_game_rate() {
    let save = std::env::temp_dir().join(format!("sml2-audio-ticks-{}.sav", std::process::id()));
    let mut game = Game::new(SessionAssets::load().unwrap(), save, false);
    let mut budget = TickBudget::default();
    let mut samples = 0;
    let mut ticks = 0;
    for elapsed in [50, 0, 16, 34] {
        for _ in 0..budget.advance(Duration::from_millis(elapsed), true) {
            game.tick(PlayerKeys::default(), 1.0);
            assert_eq!(game.buffer.len(), 735 * 2);
            samples += game.buffer.len() / 2;
            ticks += 1;
        }
    }
    assert_eq!(ticks, 6);
    assert_eq!(game.session.sound_driver.update_count, 6);
    assert_eq!(samples, 4410, "100 ms of gameplay produces 100 ms of PCM");
}
