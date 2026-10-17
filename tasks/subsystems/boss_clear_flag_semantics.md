# Remove inferred boss-clear reward handoff

Source correction: sBossClearFlag's sole ROM read is the countdown suppression
in COMPLETE tasks/handlers/0x05D6A.md. Source 05E12 tests phase3, not clear flag.
Current PlayerSim::apply_object_result invents a reward handoff when clear=true,
fade0 and boss_def_index!=0; this interrupts Wario's final animation/ending and
can reward a coin without its source terminal producer.

Keep the existing explicit obj.boss_clear_handoff producer from Mario Zone's
reward.rs; remove ONLY the inferred fallback. No phase/level special-case guard.
Read native guide/PORTING, named sourcebriefs, bounded apply_object_result and
player_boss_clear_tests.rs, Mario Zone reward's explicit event. Touch ONLY
player_sim.rs receiver condition + stale boss_clear field comment, and bounded
player_boss_clear_tests.rs source regressions/false inferred tests if present.
Do not edit runtime/ending worker or other gameplay handlers.

First show new regression fails current code: object clear flag -> player
receiver with fade0, defeat music24, Wario definition6/7/8; music/exit/progress
unchanged and countdown remains frozen. Also flag-only normal definitions have
no inferred coin award. Explicit boss_clear_handoff still applies correct reward
once, existing producer/consumer tests retained. Tests that assert fabricated
fallback must be corrected against source, not silently weakened.
Full core/runtime, sensitivity fail+restore. Ask root before mutation/fullsuite;
form3 source worker owns lane. No commit/sweep/queue/inventory edits.
