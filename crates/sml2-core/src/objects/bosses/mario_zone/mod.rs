//! Mario Zone boss (Three Piggys, Def 2), bank 2.
//!
//! Sub-objects:
//! - Parts (anims 10 & 11), controller (anim 12), spawner (anim 2) and the
//!   controller's child (anim 13): native `pigs.rs`, boundary `pigs_adapter.rs`
//! - Reward chain (anims 4 & 5): `reward.rs`

mod pig2_motion;
mod pigs;
mod pigs_adapter;
mod reward;
mod reward_motion;
#[cfg(test)]
#[path = "tests/integration.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/pigs.rs"]
mod pigs_tests;

#[cfg(test)]
#[path = "tests/pig2_motion.rs"]
mod pig2_motion_tests;
