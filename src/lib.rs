#![no_std]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![doc = include_str!("../README.md")]

#[cfg(test)]
extern crate std;

mod decode;
mod encode;
mod error;
mod read;
mod write;
pub use decode::{Decode, DecodeIter, DecodeIterator};
pub use encode::Encode;
pub use error::{Incomplete, InsufficientBuffer, InvalidWidth, TrailingBytes};
pub use read::{read_array, read_be_uint, read_u8, read_u16_be, read_u32_be, read_u64_be, take};
pub use write::{write_all, write_be_uint, write_u8, write_u16_be, write_u32_be, write_u64_be};
