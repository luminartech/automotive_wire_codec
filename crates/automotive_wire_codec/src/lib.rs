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
pub use error::{Incomplete, TrailingBytes};
