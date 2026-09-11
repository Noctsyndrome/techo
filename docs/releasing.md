# Releasing techō

Crates.io releases are deliberate, manual milestones. A published version is
immutable: do not publish a version that has not been reviewed and tested.

## Before a release

1. Choose the next [Semantic Versioning](https://semver.org/) version in
   `Cargo.toml`. Do not reuse a version already on Crates.io.
2. Update `Cargo.lock` when dependency resolution changes, and review the
   package metadata: description, license, repository, keywords, and
   categories.
3. Commit the intended release contents to `main` and push them. `cargo
   publish` intentionally refuses a dirty Git working tree.
4. Make sure the publisher has run `cargo login` and has a verified email
   address on Crates.io. Never put an API token in this repository.

## Verify

Run the following from the repository root:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
cargo package --list
cargo publish --dry-run
```

On Windows, `dev.ps1` loads the Visual C++ build environment. Its Clippy call
uses an argument array so PowerShell preserves Cargo's lint separator:

```powershell
.\dev.ps1 fmt --check
.\dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')
.\dev.ps1 test --locked
.\dev.ps1 build --locked
.\dev.ps1 package --list
.\dev.ps1 publish --dry-run
```

Review `cargo package --list` before the dry run so no unintended file is
included. The dry run builds the packaged crate in an isolated directory.

Run the manual GitHub Actions workflow as a final cross-platform check. It
runs the verification suite on Linux, macOS and Windows, including the PTY
smoke test on Linux and macOS. Wait for a successful result before publishing.

## Publish

After the checks and workflow pass, with a clean working tree:

```sh
cargo publish
```

Confirm the new version is listed at
[`crates.io/crates/techo`](https://crates.io/crates/techo), then verify the
installation in a fresh environment:

```sh
cargo install techo --version <version>
techo --help
```

Do not create a pull request or a release tag as part of this process unless
explicitly requested. This personal project publishes from the current `main`
branch.
