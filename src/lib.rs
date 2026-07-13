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
//! ```rust
//! # use automotive_wire_codec::{read_u8, read_u16_be, Decode, Incomplete, TrailingBytes};
//! # struct SomeType(u8);
//! # #[derive(Debug)]
//! # enum DemoErr { Incomplete(Incomplete), TrailingBytes(TrailingBytes) }
//! # impl From<Incomplete> for DemoErr { fn from(e: Incomplete) -> Self { DemoErr::Incomplete(e) } }
//! # impl From<TrailingBytes> for DemoErr { fn from(e: TrailingBytes) -> Self { DemoErr::TrailingBytes(e) } }
//! # impl<'a> Decode<'a> for SomeType {
//! #     type Error = DemoErr;
//! #     fn decode(buf: &'a [u8]) -> Result<(Self, &'a [u8]), Self::Error> {
//! #         let (b, rest) = read_u8(buf)?;
//! #         Ok((SomeType(b), rest))
//! #     }
//! # }
//! # fn run(buf: &[u8]) -> Result<(), DemoErr> {
//! let (a, rest) = read_u8(buf)?;
//! let (b, rest) = read_u16_be(rest)?;
//! let (c, rest) = SomeType::decode(rest)?;   // nested Decode composes the same way
//! # let _ = (a, b, c, rest);
//! # Ok(())
//! # }
//! # run(&[0u8, 1, 2, 3, 4]).unwrap();
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
//! ```rust
//! # use automotive_wire_codec::{write_u32_be, Encode};
//! # struct Header { payload_len: u32 }
//! # impl Header {
//! #     fn new(payload_len: u32) -> Self { Header { payload_len } }
//! # }
//! # impl Encode for Header {
//! #     type Error = embedded_io::ErrorKind;
//! #     fn encoded_size(&self) -> usize { 4 }
//! #     fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
//! #         write_u32_be(writer, self.payload_len)
//! #     }
//! # }
//! # struct Inner(u32);
//! # impl Encode for Inner {
//! #     type Error = embedded_io::ErrorKind;
//! #     fn encoded_size(&self) -> usize { 4 }
//! #     fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
//! #         write_u32_be(writer, self.0)
//! #     }
//! # }
//! # let inner = Inner(42);
//! # let mut tx_buf = [0u8; 8];
//! let payload_len = inner.encoded_size();
//! let header = Header::new(payload_len as u32);
//!
//! let mut writer: &mut [u8] = &mut tx_buf;       // one buffer
//! let mut total = header.encode(&mut writer)?;   // writes header, advances `writer`
//! total += inner.encode(&mut writer)?;           // writes inner into the remainder
//! # assert_eq!(total, 8);
//! # Ok::<(), embedded_io::ErrorKind>(())
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
