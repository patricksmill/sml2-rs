# Live player screen-X input for boss targeting

Shared input prerequisite for Wario23. COMPLETE source
[`$54A7` bank1 player projection](../handlers/0x054A7.md) writes screenX as
wrapped low-byte (player world X - camera X +96), before boss object handling.
Native runtime already calls CameraSim draw/timer phase before objects and
passes current player contact + camera coordinates to ObjectSim::step.
Boss engine currently ignores camera args; no boss target screen snapshot exists.

Read native guide/PORTING, named brief, bounded ObjectSim fields/new/step in
objects/mod.rs and boss_engine in objects/boss.rs, and bounded existing object
engine integration test patterns. Touch ONLY mod.rs field/default/test-module
registration, boss.rs boss_engine head (rename _cam_x and derive input), new
objects/boss_screen_input_tests.rs. No other helper/body changes.

Add descriptive input `player_screen_x: u8` to ObjectSim; derive from current
self.mario.x + supplied cam_x at boss-engine entry, using native coordinate math
with source projection wrapping. Initial input64 matches `$0F2A`; subsequent
boss frames derive independently (replay samples are never injected). Do not
add raw register/scratch fields or read console trajectories. Dead dispatch
stays frozen. Normal caller tests must exercise ObjectSim::step -> boss_engine
with a real MarioContact and F0 scene flag, camera movement and world low-byte
wrap; literal expected snapshots (e.g. player100,cam80 =>116).

Verify current boss dispatch freeze and core suite; sensitivity mutation fail
and restore. No commits/sweep/queue/inventory edits; root handles acceptance.
