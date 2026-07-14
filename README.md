# automotive-wire-codec

[![CI](https://github.com/luminartech/automotive_wire_codec/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/luminartech/automotive_wire_codec/actions/workflows/main.yml)
[![Crates.io](https://img.shields.io/crates/v/automotive-wire-codec.svg)](https://crates.io/crates/automotive-wire-codec)
[![docs.rs](https://img.shields.io/docsrs/automotive-wire-codec)](https://docs.rs/automotive-wire-codec)
[![codecov](https://codecov.io/gh/luminartech/automotive_wire_codec/branch/main/graph/badge.svg)](https://codecov.io/gh/luminartech/automotive_wire_codec)
[![License](https://img.shields.io/crates/l/automotive-wire-codec.svg)](#license)

The **L0 foundation** of a layered, `no_std`, no-alloc automotive diagnostic protocol
suite (`DoIP`, UDS, later SOME/IP). It provides the shared zero-copy codec traits and
big-endian byte-level leaf helpers that every protocol core (L1) implements — and
nothing else: no framing, no concrete message types, no owned forms, no `alloc`.

## Features

- **Zero-copy decode** — \[`Decode`\] borrows directly from the input buffer; no
  allocation, no intermediate copies.
- **`no_std` / no-alloc** — builds on bare-metal targets (verified in CI against
  `thumbv6m-none-eabi`).
- **Nested encode without a staging buffer** — \[`Encode::encoded_size`\] is exact, so an
  outer protocol can size a header and serialize an inner value directly into the same
  buffer.
- **Generic, ergonomic errors** — L1 crates keep their own rich error enum; leaf helpers
  and trait defaults construct errors generically via small `From` bounds, so calls
  compose through `?` with no turbofish.

## Error model

L0 defines no protocol error type. It defines two tiny error *fragments* —
\[`Incomplete`\] (a read ran out of bytes) and \[`TrailingBytes`\] (bytes remained after
an exact decode) — and the traits require the L1 error to be constructible `From`
them. This preserves each L1 crate's rich, typed error enum while letting shared
trait defaults and leaf helpers construct errors generically. Encode-side I/O
failures surface as \[`embedded_io::ErrorKind`\]; the \[`Encode`\] error bound is
`From<embedded_io::ErrorKind>`. Because the L1 error implements these `From` bounds,
helper calls (`read_u8(buf)?`, `write_u16_be(w, x)?`) compose through `?` with no
turbofish and no generic error parameter at the call site.

## The `decode` / `decode_exact` contract

\[`Decode::decode`\] consumes from the **front** of the buffer and returns the
**remainder**, so nested and sequential decodes thread the remainder along:

```rust
use automotive_wire_codec::{read_u8, read_u16_be, Decode, Incomplete, TrailingBytes};
struct SomeType(u8);
#[derive(Debug)]
enum DemoErr { Incomplete(Incomplete), TrailingBytes(TrailingBytes) }
impl From<Incomplete> for DemoErr { fn from(e: Incomplete) -> Self { DemoErr::Incomplete(e) } }
impl From<TrailingBytes> for DemoErr { fn from(e: TrailingBytes) -> Self { DemoErr::TrailingBytes(e) } }
impl<'a> Decode<'a> for SomeType {
    type Error = DemoErr;
    fn decode(buf: &'a [u8]) -> Result<(Self, &'a [u8]), Self::Error> {
        let (b, rest) = read_u8(buf)?;
        Ok((SomeType(b), rest))
    }
}
fn run(buf: &[u8]) -> Result<(), DemoErr> {
    let (a, rest) = read_u8(buf)?;
    let (b, rest) = read_u16_be(rest)?;
    let (c, rest) = SomeType::decode(rest)?;   // nested Decode composes the same way
    let _ = (a, b, c, rest);
    Ok(())
}
run(&[0u8, 1, 2, 3, 4]).unwrap();
```

\[`Decode::decode_exact`\] instead requires the whole buffer to be consumed, returning
\[`TrailingBytes`\] otherwise — use it at a message boundary where framing has already
delimited the frame. L0 has no opinion on framing; that is an L1 concern.

## Nested encode with no staging buffer

Because \[`Encode::encoded_size`\] is separate from \[`Encode::encode`\] and
`&mut [u8]` is an \[`embedded_io::Write`\] sink, an outer protocol serializes an inner
value directly into one buffer — no second allocation or copy:

```rust
use automotive_wire_codec::{write_u32_be, Encode};
struct Header { payload_len: u32 }
impl Header {
    fn new(payload_len: u32) -> Self { Header { payload_len } }
}
impl Encode for Header {
    type Error = embedded_io::ErrorKind;
    fn encoded_size(&self) -> usize { 4 }
    fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
        write_u32_be(writer, self.payload_len)
    }
}
struct Inner(u32);
impl Encode for Inner {
    type Error = embedded_io::ErrorKind;
    fn encoded_size(&self) -> usize { 4 }
    fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
        write_u32_be(writer, self.0)
    }
}
let inner = Inner(42);
let mut tx_buf = [0u8; 8];
let payload_len = inner.encoded_size();
let header = Header::new(payload_len as u32);

let mut writer: &mut [u8] = &mut tx_buf;       // one buffer
let mut total = header.encode(&mut writer)?;   // writes header, advances `writer`
total += inner.encode(&mut writer)?;           // writes inner into the remainder
assert_eq!(total, 8);
Ok::<(), embedded_io::ErrorKind>(())
```

## Usage

Add the dependency:

```sh
cargo add automotive-wire-codec
```

Implement \[`Encode`\] and \[`Decode`\] for a message type using the big-endian leaf
helpers:

```rust
use automotive_wire_codec::{read_u16_be, write_u16_be, Decode, Encode, Incomplete, TrailingBytes};

#[derive(Debug, PartialEq)]
struct Ping {
    session_id: u16,
}

#[derive(Debug)]
enum PingError {
    Incomplete(Incomplete),
    TrailingBytes(TrailingBytes),
    Io(embedded_io::ErrorKind),
}
impl From<Incomplete> for PingError {
    fn from(e: Incomplete) -> Self {
        PingError::Incomplete(e)
    }
}
impl From<TrailingBytes> for PingError {
    fn from(e: TrailingBytes) -> Self {
        PingError::TrailingBytes(e)
    }
}
impl From<embedded_io::ErrorKind> for PingError {
    fn from(e: embedded_io::ErrorKind) -> Self {
        PingError::Io(e)
    }
}

impl<'a> Decode<'a> for Ping {
    type Error = PingError;
    fn decode(buf: &'a [u8]) -> Result<(Self, &'a [u8]), Self::Error> {
        let (session_id, rest) = read_u16_be(buf)?;
        Ok((Ping { session_id }, rest))
    }
}

impl Encode for Ping {
    type Error = PingError;
    fn encoded_size(&self) -> usize {
        2
    }
    fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
        Ok(write_u16_be(writer, self.session_id)?)
    }
}

fn main() -> Result<(), PingError> {
    // Round-trip: encode into a buffer, then decode it back.
    let ping = Ping { session_id: 0x1234 };
    let mut buf = [0u8; 2];
    let mut writer: &mut [u8] = &mut buf;
    ping.encode(&mut writer)?;

    let decoded = Ping::decode_exact(&buf)?;
    assert_eq!(decoded, ping);
    Ok(())
}
```

See the [crate docs](https://docs.rs/automotive-wire-codec) for the full API,
including the \[`DecodeIter`\] trait for repeated elements and the variable-width
\[`read_be_uint`\] helper.

## `no_std`

This crate is `no_std` and does not require `alloc`. `unsafe_code` is forbidden
(`#![forbid(unsafe_code)]` at the workspace lint level). CI builds against a bare-metal
Cortex-M0 target (`thumbv6m-none-eabi`) to catch any `std`/`alloc` leaking in through a
dependency.

## Minimum Supported Rust Version (MSRV)

The MSRV is tracked in `Cargo.toml`'s `rust-version` field (currently 1.85) and enforced
in CI.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
