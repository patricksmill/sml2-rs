use super::*;

#[test]
fn slow_playback_keeps_recent_events_instead_of_old_audio() {
    let queue = SampleQueue::default();
    for event in [1.0, 2.0, 3.0] {
        queue.push(vec![event; LIVE_FRAME_SAMPLES * 2]);
    }
    assert!(queue.pop().unwrap().iter().all(|&s| s == 2.0));
    assert!(queue.pop().unwrap().iter().all(|&s| s == 3.0));
    queue.close();
    assert!(queue.pop().is_none());
}

#[test]
fn a_hitch_sized_chunk_cannot_expand_the_playback_backlog() {
    let queue = SampleQueue::default();
    queue.push((0..MAX_SAMPLES * 3).map(|n| n as f32).collect());
    let samples = queue.pop().unwrap();
    assert_eq!(samples.len(), MAX_SAMPLES);
    assert_eq!(samples[0], (MAX_SAMPLES * 2) as f32);
    assert_eq!(samples[MAX_SAMPLES - 1], (MAX_SAMPLES * 3 - 1) as f32);
}

#[test]
fn pausing_discards_pending_audio_and_rejects_further_writes() {
    let queue = SampleQueue::default();
    queue.push(vec![1.0; LIVE_FRAME_SAMPLES * 2]);
    queue.close();
    queue.push(vec![2.0; LIVE_FRAME_SAMPLES * 2]);
    assert!(queue.pop().is_none());
}
