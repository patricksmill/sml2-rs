//! Mario Zone boss (Three Piggys, Def 2), bank 2.
//!
//! Sub-objects:
//! - Parts (anims 10 & 11): `part.rs`
//! - Controller (anim 12): `controller.rs`
//! - Spawner (anim 2): `spawner.rs`
//! - Child (anim 13): `child.rs`
//! - Reward chain (anims 4 & 5): `reward.rs`

mod child;
mod controller;
mod part;
mod reward;
mod reward_motion;
mod spawner;
#[cfg(test)]
mod tests;
