# Migrating a protocol crate onto automotive-wire-codec

Field guide distilled from the uds/doip/someip migrations. Applies to 0.3.

## The error pattern: one crate-wide error absorbs the fragments

Give your existing crate-wide error enum one `From` impl per codec fragment:

```rust
impl From<automotive_wire_codec::Incomplete> for Error { /* variant */ }
impl From<automotive_wire_codec::TrailingBytes> for Error { /* variant */ }
impl From<embedded_io::ErrorKind> for Error { /* variant */ }
// If you use the variable-width helpers, also:
impl From<automotive_wire_codec::ReadUintError> for Error { /* match both arms */ }
impl From<automotive_wire_codec::WriteUintError> for Error { /* match both arms */ }
```

Then every `impl Decode`/`Encode` block is `type Error = crate::Error;` and all
leaf-helper calls lift through `?`. **Do not mint a per-type error enum for
each impl** — besides ~20 lines of boilerplate each, any type used as an
associated `Error` in a `pub` trait impl on a `pub` type must itself be `pub`
(rustc E0446; `pub(crate)` is not sufficient), so every bespoke error becomes
new public API surface.

`thiserror` (v2, `default-features = false`) with `#[error(transparent)]` +
`#[from]` wrapping the fragments compiles clean under `no_std`.

## Why there is no blanket `impl Decode for u16`

A single primitive impl would have to pick one error type for everyone. The
associated `Error` is the load-bearing design decision; the `read_u16_be`-style
leaf helpers are the substitute. Expect per-type impls.

## Dual-trait coexistence during migration

Old concrete-error trait and the codec traits can live on the same type while
you migrate callers module by module — they don't collide:

```rust
impl WireFormat for Header { /* legacy, deleted last */ }
impl<'a> automotive_wire_codec::Decode<'a> for Header { type Error = Error; /* new */ }
```

Cut callers over incrementally; delete the legacy trait when `grep` says nobody
implements or calls it.

## Where to put `Decode` impls for invariant-bearing types

An impl outside the type's defining module cannot use `Self { .. }` on private
fields, and routing through a validating constructor may *recompute* state the
wire declared (e.g. re-deriving minimal widths a sender over-declared),
silently changing bytes on re-encode. Per invariant-bearing type, choose
explicitly: (1) impls in the defining module, (2) a `pub(crate)` raw
constructor that preserves wire-declared state, or (3) accept recomputation —
only safe when the wire format has no redundant-encoding freedom.

## Behavior changes to audit when replacing hand-rolled code

- **`encoded_size()` is `Result` and exact.** The default (counting sink) is
  correct by construction. If you override, `written == encoded_size()?` is a
  hard invariant — the strictness is a feature; it has caught real
  compensating size bugs at migration time. Do NOT drop your override if
  `encode` calls `self.encoded_size()` (self-length prefixes): the default
  runs `encode` to count, so that pair recurses infinitely. Keep closed-form
  overrides there and on hot paths (the default costs a full encode pass).
- **Per-element iterator errors.** `DecodeIterator` surfaces a malformed
  element as `Some(Err(_))` and then fuses (yields `None` forever). Iterators
  that silently stopped on truncation, or that repeated the same error forever,
  change observable behavior — audit callers.
- **Per-field `Incomplete` counts.** Chained leaf reads report
  `needed`/`available` for the *first field* that underruns; an up-front
  whole-record length check reports the *record* size. Callers that
  pattern-match on `Incomplete.needed` (e.g. partial-buffer reassembly) must be
  audited.
- **`Send`/`Sync`.** The codec traits carry no such bounds (embedded-first). If
  a legacy trait provided them for free, restate the bound at spawn-adjacent
  call sites: `where T: Encode + Send + 'static`. Concrete owned-field message
  types remain auto-`Send + Sync`.
- **Hostile widths.** `read_be_uint`/`write_be_uint` return
  `InvalidWidth` for `n > 16` in every profile — delete any upstream
  `n > 16` guards you carried, or keep them for domain-specific narrower
  bounds (`read_be_uint_into::<u32>` enforces `n <= 4` for you). Zero width is
  legal (reads/writes nothing); protocols requiring `n >= 1` must still check.

## Fixed-buffer encoding

Use `value.encode_to_slice(&mut buf)?` — a too-small buffer reports
`InsufficientBuffer { needed, available }` (sized via `encoded_size()` on the
failure path only; success is a single encode pass). Encoding through a raw
`&mut [u8]` sink instead surfaces plain `ErrorKind::WriteZero` with no counts.
