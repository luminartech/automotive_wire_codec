#![no_std]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

//! **L0 foundation** of a layered, `no_std`, no-alloc automotive diagnostic protocol
//! suite. Provides the shared zero-copy codec traits ([`Encode`], [`Decode`],
//! [`DecodeIter`]) and the big-endian byte-level leaf helpers every protocol core (L1)
//! is built from. It defines no framing, no concrete message types, and no owned forms.

#[cfg(test)]
extern crate std;

mod error;
mod read;
mod write;
pub use error::{Incomplete, TrailingBytes};
pub use read::{read_array, read_be_uint, read_u16_be, read_u32_be, read_u64_be, read_u8, take};
pub use write::{write_all, write_be_uint, write_u16_be, write_u32_be, write_u64_be, write_u8};
