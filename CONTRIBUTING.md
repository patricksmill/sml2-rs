# Contributing

Thanks for helping. This is a native Rust port; read [PORTING.md](PORTING.md)
before changing game logic, and use the [issue forms](.github/ISSUE_TEMPLATE)
to report bugs or propose features.

## Build and run

```sh
cargo run -p sml2-desktop
```

The game needs your own **Super Mario Land 2 UE v1.0** ROM; import it with
**Settings → Game → Import ROM…**. Linux build packages are listed in
[docs/desktop.md](docs/desktop.md#build-and-run).

## Checks before opening a pull request

```sh
cargo test -p sml2-core
```

That is the test gate: every ported routine keeps its byte-level tests passing.
Runtime and parity tests that read extracted game data only run locally.

## Never commit game data

Do not commit ROM files, `extracted/`, imported graphics, music or any other
derived game data. They are copyrighted and must stay on your machine; the
repository and releases contain only this project's own code and art.
`extracted/` is git-ignored for that reason.
