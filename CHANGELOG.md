# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0]

### Breaking

- `Encode::encode` takes `&mut impl Sink`, not `&mut impl embedded_io::Write`.
- `Encode::Error` bound is `From<WriteError>`, not `From<embedded_io::ErrorKind>`.
- `EncodeToSliceError` deleted. `encode_to_slice` returns `Self::Error`.
- `WriteUintError` deleted. `WriteError` carries `InvalidWidth`.
- Free function `write_all` renamed `write_bytes` (collided with `Sink::write_all`).
- Write helpers take `&mut impl Sink` and return `WriteError`.
- `embedded-io` removed entirely — no dependency, no feature, no blanket impl.
  A consumer holding an `embedded_io::Write` peripheral writes a ten-line `Sink` impl.
- `&mut [u8]` is no longer a sink. `let mut w: &mut [u8] = &mut buf; v.encode(&mut w)`
  (via embedded-io's slice impl) no longer compiles. Use
  `v.encode(&mut SliceSink::new(&mut buf))`, or `v.encode_to_slice(&mut buf)`. See
  `MIGRATION.md` step 7.
- `InsufficientBuffer::needed`'s **value** changed, not just its documentation. In
  0.3 it was `encoded_size()` — an exact total. In 0.4 it is `written + buf.len()`
  at the failing write — a lower bound, always ≤ the 0.3 value, with no compile
  error to flag the change. Code sizing a retry buffer from `needed` is now
  silently wrong; call [`encoded_size()`](crate::Encode::encoded_size) for an
  exact total instead.

### Added

- `Sink`, with `remaining()` reporting guaranteed capacity (`usize::MAX` = unbounded).
- `SliceSink`, exact capacity, backs `encode_to_slice`.
- `Limited`, bounding any sink to a byte budget — how a transport enforces its
  advertised maximum so an over-long encode fails with counts attached.
- `WriteError`, one error type for the whole write path.
- `impl<S: Sink + ?Sized> Sink for &mut S` — lets a function holding
  `sink: &mut impl Sink` wrap its own borrow (`Limited::new(&mut *sink, n)`) and
  lets `&mut dyn Sink` satisfy an `impl Sink` bound.

### Changed

- `CountingSink` implements `Sink`; same behaviour, same public path.
- `encode_to_slice` is single-pass on the failure path as well as on success.

### Unchanged

- The entire read side, and `no_std` / no-alloc / zero-copy.

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
