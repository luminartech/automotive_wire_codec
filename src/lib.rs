#![no_std]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

//! # automotive-wire-codec — L0
//!
//! The **L0 foundation** of a layered, `no_std`, no-alloc automotive diagnostic protocol
//! suite (`DoIP`, UDS, later SOME/IP). It provides the shared zero-copy codec traits and
//! big-endian byte-level leaf helpers that every protocol core (L1) implements — and
//! nothing else: no framing, no concrete message types, no owned forms, no `alloc`.
//!
//! ## Error model
//!
//! L0 defines no protocol error type. It defines two tiny error *fragments* —
//! [`Incomplete`] (a read ran out of bytes) and [`TrailingBytes`] (bytes remained after
//! an exact decode) — and the traits require the L1 error to be constructible `From`
//! them. This preserves each L1 crate's rich, typed error enum while letting shared
//! trait defaults and leaf helpers construct errors generically. Encode-side I/O
//! failures surface as [`embedded_io::ErrorKind`]; the [`Encode`] error bound is
//! `From<embedded_io::ErrorKind>`. Because the L1 error implements these `From` bounds,
//! helper calls (`read_u8(buf)?`, `write_u16_be(w, x)?`) compose through `?` with no
//! turbofish and no generic error parameter at the call site.
//!
//! ## The `decode` / `decode_exact` contract
//!
//! [`Decode::decode`] consumes from the **front** of the buffer and returns the
//! **remainder**, so nested and sequential decodes thread the remainder along:
//!
//! ```text
//! let (a, rest) = read_u8(buf)?;
//! let (b, rest) = read_u16_be(rest)?;
//! let (c, rest) = SomeType::decode(rest)?;   // nested Decode composes the same way
//! ```
//!
//! [`Decode::decode_exact`] instead requires the whole buffer to be consumed, returning
//! [`TrailingBytes`] otherwise — use it at a message boundary where framing has already
//! delimited the frame. L0 has no opinion on framing; that is an L1 concern.
//!
//! ## Nested encode with no staging buffer
//!
//! Because [`Encode::encoded_size`] is separate from [`Encode::encode`] and
//! `&mut [u8]` is an [`embedded_io::Write`] sink, an outer protocol serializes an inner
//! value directly into one buffer — no second allocation or copy:
//!
//! ```text
//! let payload_len = inner.encoded_size();
//! let header = Header::new(/* ... */, payload_len as u32);
//!
//! let mut writer: &mut [u8] = &mut tx_buf;       // one buffer
//! let mut total = header.encode(&mut writer)?;   // writes header, advances `writer`
//! total += inner.encode(&mut writer)?;           // writes inner into the remainder
//! ```

#[cfg(test)]
extern crate std;

mod decode;
mod encode;
mod error;
mod read;
mod write;
pub use decode::{Decode, DecodeIter, DecodeIterator};
pub use encode::Encode;
pub use error::{Incomplete, TrailingBytes};
pub use read::{read_array, read_be_uint, read_u8, read_u16_be, read_u32_be, read_u64_be, take};
pub use write::{write_all, write_be_uint, write_u8, write_u16_be, write_u32_be, write_u64_be};
