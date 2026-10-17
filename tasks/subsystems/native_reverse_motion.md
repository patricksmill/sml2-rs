# Native reverse boss motion profiles

One shared movement-profile subsystem, ROM `$B1C8/$B1D6/$B1E4` bank2 and
37-byte table `$B1FF..$B223`, COMPLETE brief `tasks/handlers/0x0B1C8.md`
and existing exact byte tests in `objects/boss_reverse_picker.rs`.
ROM table starts with full profile at offset0 cap36; medium offset15 cap21;
short offset24 cap12. Read current phase before increment, equality-only cap,
phase increment wraps, return literal distance. Existing Actor::motion_step
already owns that behavior; reuse it. No second copy of the table.

Create descriptive typed rise profile enum (Full/Medium/Short or equivalent)
with cap and slice mapping, reusable `Actor::rise_step(profile)` returning
Result<u8, MotionProfileError>. Move existing single literal table to native
module; old three facade methods convert Actor and delegate while preserving
(step,cap) and existing valid-range tests. The old generic picker fabricated
zero and incremented for invalid phases; intentionally reject unsupported
source phases with a facade panic, while the native API returns an error
without mutation. Add a regression for this invalid-data rejection.

Read PORTING/native guide, named brief and `actor.rs`, existing reverse picker,
and lib.rs registration if needed. Touch ONLY actor.rs (small method/module
registration), new bounded actor motion sibling and optional tests, existing
boss_reverse_picker.rs replacement facade and retained tests; lib.rs registration
only if needed. No handler bodies, queue/inventory or unrelated helper edits.
Validate literal profile independently against ROM; native tests for all three
profiles and boundary/wrapping/error cases plus unchanged existing byte tests.
Full core tests; ask root for sensitivity window. No sweep/commit. Native
Wario23 will use full and short profiles after acceptance.
