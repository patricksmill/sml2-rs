# Native ending sprite composition prerequisite

One shared ending graphics family: COMPLETE generated source packets
0x6845F,0x68492,0x684C5,0x684E0 in tasks/handlers/. Root independently checked
all eleven E5..EF mapping blobs/sentinels against ROM and burst lookup at68518.
Read PORTING/native guide, four packets, player.rs native append API (currently
worker-owned; coordinate root before dependent edits), object.rs mapping types,
actor.rs FrameTick/ScreenPosition bounded definitions. No runtime/object engine.

Touch ONLY new ending_sprites.rs and ending_sprites/tests.rs, lib.rs registration.
Use shared player::append_cells, CellStyle and native ScreenPosition. Provide
native pose/composition functions usable by ending controller. Walk timer is
persistent native u8: increment wrapping then if >=48 reset0; frame E5 or E8
plus timer>>4; x84,ycredits. Solo walking sets paletteone=true,behind_background=true; paired walking
sets both false (source68492 explicitly writes priority control1,palette0).
Pair routine68492 MUST append its first pose then FALL THROUGH secondary684C5.
Secondary uses EB,x79,y=credits+52 wrapping, paletteone=false and INHERITS
priority: explicit argument/style, no scratch register state. Burst uses exact
[EB,EC,ED,EE,EF,EC,ED,EE] by (frame&1C)>>2; x79,y96, paletteone=false,
behind_background=false. Odd frame writes request1=24; expose typed effect
for future sound consumer; no sound tick/acknowledgment/no-op driver invention.

Keep domain concise and use existing extracted tables, no duplicate geometry,
raw HRAM/bank/register simulation, or inferred sequence dispatch. Missing assets
must return explicit failure rather than silently partial success. Source order
for timer/effect before failed append should be deliberate and documented.

Tests exact full rendered bytes with literal ROM records, timer boundaries
0/14/15/30/31/46/47/48/254/255, wraps, existing prefix and pair ordering,
inherited priority both values, full frame byte burst cadence/table and sound
effect ordering. Test shared append consumer directly. Root coordinates full
suite/mutations: focused tests allowed; ask root before taking lane. No commits,
sweep/queue/inventory/captures edits. Scene/controller/render integration remains
pending; this is its reusable verified graphics prerequisite.
