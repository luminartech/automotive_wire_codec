# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/luminartech/automotive_wire_codec/compare/v0.2.0...v0.3.0) - 2026-07-15

Addresses the aggregated migration feedback from the uds, doip, and someip
protocol crates.

### Breaking

- `Encode::encoded_size` now returns `Result<usize, Self::Error>` and has a
  correct-by-construction default that counts bytes through `CountingSink`.
  Existing overrides: wrap the returned size in `Ok(..)` — or delete the
  override and take the default. **Caveat:** do NOT delete the override if
  your `encode` calls `self.encoded_size()` (e.g. to write a self-length
  prefix) — the default is implemented by running `encode` against a
  counting sink, so that combination recurses infinitely at runtime.
  Keep a closed-form override for such types (and for hot paths, where the
  counting default means sizing costs a full encode pass).
- `read_be_uint` returns `Result<_, ReadUintError>` and `write_be_uint`
  returns `Result<_, WriteUintError>`: width is now a checked *data* error
  (`InvalidWidth`) in all build profiles. This also fixes a release-build
  panic in `write_be_uint` for `n > 16` (slice-index underflow).

### Added

- `Encode::encode_to_slice` default method + `EncodeToSliceError` +
  `InsufficientBuffer` fragment: fixed-buffer encoding with
  `needed`/`available` diagnostics and no `&mut &mut [u8]` dance.
- `CountingSink`: infallible byte-counting `embedded_io::Write` sink.
- `read_be_uint_into::<T>`: typed variable-width reader validating the width
  against the target type (no `as` casts at call sites).
- `read_u128_be` / `write_u128_be`, `ensure_len`, `read_optional_array`,
  `minimal_be_len` leaf helpers.
- `DecodeIter::WIRE_SIZE` (opt-in) + `DecodeIterator::remaining_len` for
  fixed-stride record streams.

### Documentation

- `decode_exact` multi-message boundary warning; `DecodeIter::decode_next`
  clean-end convention; `WriteZero` semantics on exhausted slice sinks.
- README "Consumer idioms" section (framing, dispatch, validated views,
  length prefixes, slice-first rationale).
- `MIGRATION.md`: error-pattern, E0446 trap, dual-trait coexistence,
  impl-placement for private-field types, behavior-change audit list.

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
