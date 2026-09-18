# Implementation Notes: Crates.io Release Process

Spec: inline user request to publish the current version and record the process.

## Design decisions

- Treat the already-versioned `0.1.0-alpha.2` on `main` as the release candidate; do not create a new version merely to document its release.
- Record the process in an English maintainer document at `docs/releasing.md`, while keeping this implementation note concise and decision-oriented.
- Document the Windows Clippy invocation with `dev.ps1 -CargoArgs @(...)`, because PowerShell otherwise consumes Cargo's `--` separator before the helper can forward lint flags.
- Gate the immutable Crates.io upload on both local package validation and the repository's manual Windows/Linux CI workflow.

## Deviations

None.

## Tradeoffs

- The release document covers the Crates.io flow now. Homebrew distribution remains outside this release because no tap or formula workflow is configured in this repository.

## Open questions

- None.
