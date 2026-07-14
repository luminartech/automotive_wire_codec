# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/luminartech/automotive_wire_codec/compare/v0.1.1...v0.2.0) - 2026-07-14

### Added

- Add CONTRIBUTING.md and SECURITY.md ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Add CODEOWNERS so required code-owner review is enforced ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Add an OpenSSF Scorecard workflow and badge ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Add keywords and categories for crates.io discoverability ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Add codecov.yml with explicit coverage gates ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))

### Changed

- Restructure CI workflows around an aggregate merge-queue gate, and pin all GitHub Actions to commit digests ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Re-run PR title/description lint on edits via a dedicated `pr-lint.yml`, instead of stale checks surviving an edited description ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Drop rust-cache from the Miri job to avoid intermittent cross-nightly cache corruption ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
- Remove polyglot firmware-template debris, unused cargo-vet config, and RUSTC_BOOTSTRAP from cargo config ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))

### Documentation

- Remove hidden line # so that the readme renders nicely ([#7](https://github.com/luminartech/automotive_wire_codec/pull/7))
- Drop the Sphinx toctree from the README and the safety stub ([#9](https://github.com/luminartech/automotive_wire_codec/pull/9))
