use super::*;
use crate::front_end::keys::{A, START, UP};

fn run(title: &mut TitleScreen, frames: u32) -> TitleAction {
    let mut action = TitleAction::None;
    for _ in 0..frames {
        action = title.step(0, 0);
    }
    action
}

#[test]
fn the_fade_timer_is_a_16_bit_countdown() {
    let mut title = TitleScreen::new();
    assert_eq!(title.timer(), 0x0540);
    title.step(0, 0);
    assert_eq!(title.timer(), 0x053F);
    for _ in 0..63 {
        title.step(0, 0);
    }
    assert_eq!(title.timer(), 0x0500);
    title.step(0, 0);
    assert_eq!(title.timer(), 0x04FF, "the low byte borrows from the high");
    run(&mut title, 0x04FF);
    assert_eq!(title.timer(), 0, "clamped at zero");
    title.step(0, 0);
    assert_eq!(title.timer(), 0, "and it stays there");
}

#[test]
fn start_is_ignored_while_the_high_byte_is_still_five() {
    let mut title = TitleScreen::new();
    for _ in 0..64 {
        assert_eq!(title.step(0, START), TitleAction::None);
    }
    assert_eq!(title.timer(), 0x0500);
    assert_eq!(
        title.step(0, START),
        TitleAction::OpenFileSelect,
        "the check is `cp $05 / ret nc`"
    );
}

#[test]
fn the_countdown_expiry_starts_the_current_demo() {
    let mut title = TitleScreen::new();
    title.set_demo_index(2);
    assert_eq!(run(&mut title, 1088), TitleAction::None);
    assert_eq!(title.timer(), 0x0100, "high is still 1");
    assert_eq!(
        title.step(0, 0),
        TitleAction::StartDemo(2),
        "the high byte reaching zero starts the demo, low byte still 255"
    );
    assert_eq!(title.timer(), 0x00FF);
}

#[test]
fn select_plus_up_starts_a_chosen_demo_early() {
    let mut title = TitleScreen::new();
    run(&mut title, 65); // high = 4, input is live
    assert_eq!(
        title.step(UP | 0x02, SELECT),
        TitleAction::StartDemo(2),
        "the held low bits select the page"
    );
    assert_eq!(title.demo_index(), 2);

    assert_eq!(title.step(UP, 0), TitleAction::None, "held is not pressed");
}

#[test]
fn only_start_opens_the_file_select() {
    let mut title = TitleScreen::new();
    run(&mut title, 65);
    assert_eq!(title.step(0, A), TitleAction::None);
    assert_eq!(title.step(0, 0x02), TitleAction::None);
    assert_eq!(title.step(0, 0x04), TitleAction::None, "SELECT alone");
    assert_eq!(title.step(0, START), TitleAction::OpenFileSelect);
}

#[test]
fn the_demo_index_cycles_through_four_pages() {
    let mut title = TitleScreen::new();
    assert_eq!(title.demo_index(), 0);
    for expected in [1, 2, 3, 0] {
        title.next_demo();
        assert_eq!(title.demo_index(), expected);
    }
    title.set_demo_index(0xFE);
    assert_eq!(title.demo_index(), 2, "the stored index is 2 bits");
}
