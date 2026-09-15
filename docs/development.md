# Development

## Build and run

```sh
cargo run --locked
```

On Windows with Visual C++ Build Tools, run `./dev.ps1` from PowerShell. It loads the MSVC environment and opens the checkout's `logs` directory as the journal. Use a standalone terminal window for interactive testing; `./scripts/dev-window.ps1` rebuilds and reopens techō in a fresh Windows Terminal window, closing the previous one first.

## Checks

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 scripts/terminal_smoke.py target/debug/techo  # Linux/macOS PTY acceptance
```

Windows: `./dev.ps1 test` and `./dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')`.

CI runs the checks on Linux, macOS and Windows, and the PTY smoke test on Linux. See [releasing.md](releasing.md) for how a version reaches Crates.io.

## Design notes

[Design review](design-review.md) sets the direction: a quiet page where anything not written by you has to earn its place. The implementation notes record each round and the reasons behind its choices:

- [interaction-feedback.md](interaction-feedback.md) and [interaction-feedback-impl-notes.md](interaction-feedback-impl-notes.md): the alpha.2 round.
- [design-review-impl-notes.md](design-review-impl-notes.md): the design review round.
- [keys-impl-notes.md](keys-impl-notes.md): the key strategy and macOS terminals.
- [release-process-impl-notes.md](release-process-impl-notes.md): the release process.
