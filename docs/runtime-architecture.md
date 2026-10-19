# Runtime boundaries

`sml2-runtime` is both a library and a CLI executable. `src/main.rs` delegates to
`run_cli`; a frontend can drive a session without invoking argument parsing,
opening a window/audio device, or providing a save path.

## Ownership

| Owner | Responsibility |
|---|---|
| `content::{ContentSource, ExtractedContent, SessionAssets}` | Read and decode content before simulation starts |
| `session::{Session, LevelContext}` | Map/level transitions and native gameplay state |
| `session::{FrameInput, FrameEffects}` | Explicit tick inputs and deferred host effects |
| `render` | Read-only CPU composition, including session presentation |
| `present` | Convert logical grayscale pixels to the existing integer-scaled window buffer |
| `host::SessionRunner` | Existing CLI host: clocks, save files, audio generation/output, window and headless loops |
| `cli`, `viewer`, `image_output` | Arguments, static level viewer, PNG and audio exports |
| `play`, `replay`, `game` | Existing diagnostic play/replay adapters and shared frame-ordering helpers |

The diagnostic single-level and frozen-replay loops remain separate adapters.
They retain their existing frame ordering and verification behavior. Future
frontends should use `Session`, rather than extending those diagnostic loops.

## Loading and driving a session

```rust
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    FrameInput, Session,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = ExtractedContent::new("extracted");
    let assets = SessionAssets::load_from(&source)?;
    let mut session = Session::new(assets);

    // A host supplies normalized held buttons and the sampled DIV input.
    let effects = session.advance(FrameInput {
        keys: Default::default(),
        div: 0x5a,
    });
    for record in effects.saves {
        // Persist these exact bytes using this host's save storage.
        assert_eq!(record.len(), 0x50);
    }
    let pixels = session.render_frame((160, 144));
    assert_eq!(pixels.len(), 160 * 144);
    Ok(())
}
```

`advance` runs one tick. It never samples a clock, reads content, writes saves,
prints diagnostics, generates audio samples, or opens a device. It returns the
existing transition outcome, captured save records, and diagnostic messages.
Rendering takes an immutable reference and caller-owned viewport dimensions.

Content sources are not retained after loading. All 32 original level entries
are decoded eagerly. A missing/unusable level is cached as an error and reported
when that level is requested, preserving the previous ability to boot when an
unrelated level is unavailable. `SessionAssets::insert_level` can replace a
decoded level before constructing a session, for example for editor playtest.
An importer or memory bundle can implement `ContentSource` without writing an
`extracted/` directory. ROM-only extraction itself is still future work.

## Clock, sound and persistence contracts

The CLI host retains its existing policies: live DIV comes from the native
wall clock; headless DIV comes from the nominal 60 Hz virtual clock. New hosts
choose their clock policy and supply samples explicitly. Input-edge latches
remain in `Session`; callers submit held buttons once per tick.

The deterministic sound driver stays in `Session`. Its acknowledged song feeds
the level timer, so moving all sound state into a device backend would change
gameplay. `SessionRunner` feeds that driver into the APU, generates samples using
the previous live/headless pacing policy, and submits them to the audio output.
A new audio host must likewise ingest the sound driver after each tick, even if
it does not present an image. Rollback-aware audio remains future work.

`Session::save_record` and `apply_save_record` convert progress bytes in memory.
Autosave effects capture the record at the original transition point; a host
must persist that record rather than regenerating it from a later state.
`SessionRunner` writes effects before returning from its compatibility
`step_frame` API, preserving existing CLI save behavior. Progress records are
not complete simulation snapshots.

The existing `session::SessionRunner` and `session::AudioClock` paths are
re-exported for current callers. The runner exposes its simulation as `session`
and forwards ordinary field access. Code borrowing multiple simulation fields
should borrow them through `runner.session` explicitly.

## Verification

Keep `cargo test --workspace` green and run the frozen sweep once per work
package according to PORTING.md. The sweep checks simulation bytes, not pixels.
For renderer moves, compare native and larger-viewport PNG bytes separately.

Integration tests in `crates/sml2-runtime/tests/` exercise the public library:
injected content reaches level entry without reopening its source; explicit DIV
reaches map bookkeeping; autosaves emit ROM-format bytes; and game/sound state
agrees when rendering runs at different frequencies and sizes.

This extraction does not add window features, new desktop dependencies, binding
logic, filters, DX support, complete snapshots, or multiplayer rules. Those build
on these boundaries in the [feature roadmap](plans/2026-10-19-desktop-features-and-extensibility.md).
