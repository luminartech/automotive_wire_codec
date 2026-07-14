# Contributing

Thanks for your interest in improving `automotive-wire-codec`. This repository
is the foundation of Luminar's public automotive protocol crates, so it holds a
high verification bar — but the workflow is simple.

## Development setup

- Stable Rust via [rustup](https://rustup.rs) (the MSRV in `Cargo.toml`'s
  `rust-version` is what CI enforces).
- [pre-commit](https://pre-commit.com): `pre-commit install` once after
  cloning; the hooks also run in CI.
- Optional, for the full local suite: `cargo install cargo-nextest cargo-fuzz`
  and a nightly toolchain with the `miri` component.

## Before opening a PR

CI runs all of this; running it locally first saves round-trips:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo nextest run --all-features        # or: cargo test
cargo test --doc --all-features
cargo doc --no-deps --all-features      # doc warnings are errors in CI
pre-commit run --all-files
```

The crate is `no_std`, no-alloc, and `#![forbid(unsafe_code)]`. CI additionally
builds against a bare-metal target (`thumbv6m-none-eabi`), fuzzes the decode
path, and runs Miri — new code should keep all of those green. New public API
needs doc comments (with `# Errors` sections) and tests; decode paths must
return `Err` on malformed input, never panic.

## Pull requests

- **Squash-merge only**: the PR title becomes the commit on `main` and feeds
  the changelog, so it must follow
  [Conventional Commits](https://www.conventionalcommits.org)
  (`feat:`, `fix:`, `docs:`, `chore:`, ...). CI lints the title.
- Fill in the PR template's **Issue URL** and **Testing** sections (CI checks
  them; apply the `No Issue` label only when there genuinely is no issue).
- A code-owner review is required, and PRs merge through the merge queue once
  approved and green.

## Releases

Releases are fully automated with [release-plz](https://release-plz.dev): merged
conventional commits accumulate in a Release PR (version bump + changelog), and
merging that PR publishes to crates.io and tags the release. Don't bump
versions or edit `CHANGELOG.md` by hand.

## Security issues

See [SECURITY.md](SECURITY.md) — please don't open public issues for
suspected vulnerabilities.
