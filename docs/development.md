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
python3 scripts/terminal_smoke.py target/debug/techo  # Linux PTY acceptance
```

Windows: `./dev.ps1 test` and `./dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')`.

CI runs the checks on Linux, macOS and Windows, and the PTY smoke test on Linux. See [releasing.md](releasing.md) for how a version reaches Crates.io.

## Design notes

Start with the [documentation map](README.md). Current guides stay in `docs/`; dated reviews live in `review/`, and specs and implementation notes in `spec/`. Files stay flat within each directory; implementation notes share their source document's date and topic prefix.

The [design review](review/2026-09-11-design-review.md) records the direction of a quiet page. Read its [implementation notes](review/2026-09-11-design-review-impl-notes.md) alongside it: some proposed details were changed after hands-on feedback.

The alpha.2 round is recorded in the [interaction spec](spec/2026-09-11-interaction-feedback.md) and [implementation notes](spec/2026-09-11-interaction-feedback-impl-notes.md). Other records cover the [key strategy and macOS terminals](spec/2026-09-15-keys-impl-notes.md) and the [release process](spec/2026-09-11-release-process-impl-notes.md).

The [initial release readiness review](review/2026-09-18-initial-release-readiness.md) is advisory, not an approved implementation plan. Check its baseline and the current task before acting on a recommendation.
