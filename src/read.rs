//! Big-endian, `core`-only slice read helpers. Each returns `(value, remainder)` so
//! callers thread the remainder through sequential/nested decodes.

use crate::error::{Incomplete, InvalidWidth};

/// Split `n` bytes off the front of `buf`, returning `(head, tail)`.
///
/// # Errors
/// [`Incomplete`] if `buf` has fewer than `n` bytes.
pub fn take(buf: &[u8], n: usize) -> Result<(&[u8], &[u8]), Incomplete> {
    if buf.len() < n {
        Err(Incomplete {
            needed: n,
            available: buf.len(),
        })
    } else {
        Ok(buf.split_at(n))
    }
}

/// Read a single byte.
///
/// # Errors
/// [`Incomplete`] if `buf` is empty.
pub fn read_u8(buf: &[u8]) -> Result<(u8, &[u8]), Incomplete> {
    let (b, rest) = take(buf, 1)?;
    Ok((b[0], rest))
}

/// Read a big-endian `u16`.
///
/// # Errors
/// [`Incomplete`] if fewer than 2 bytes remain.
pub fn read_u16_be(buf: &[u8]) -> Result<(u16, &[u8]), Incomplete> {
    let (b, rest) = take(buf, 2)?;
    Ok((u16::from_be_bytes([b[0], b[1]]), rest))
}

/// Read a big-endian `u32`.
///
/// # Errors
/// [`Incomplete`] if fewer than 4 bytes remain.
pub fn read_u32_be(buf: &[u8]) -> Result<(u32, &[u8]), Incomplete> {
    let (b, rest) = take(buf, 4)?;
    Ok((u32::from_be_bytes([b[0], b[1], b[2], b[3]]), rest))
}

/// Read a big-endian `u64`.
///
/// # Errors
/// [`Incomplete`] if fewer than 8 bytes remain.
pub fn read_u64_be(buf: &[u8]) -> Result<(u64, &[u8]), Incomplete> {
    let (b, rest) = take(buf, 8)?;
    Ok((
        u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        rest,
    ))
}

/// Read a fixed-size `N`-byte array (e.g. a 17-byte VIN, a 6-byte EID).
///
/// # Errors
/// [`Incomplete`] if fewer than `N` bytes remain.
pub fn read_array<const N: usize>(buf: &[u8]) -> Result<([u8; N], &[u8]), Incomplete> {
    let (b, rest) = take(buf, N)?;
    let mut arr = [0u8; N];
    arr.copy_from_slice(b);
    Ok((arr, rest))
}

/// Error from the variable-width read helpers ([`read_be_uint`],
/// `read_be_uint_into`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadUintError {
    /// Not enough input bytes.
    Incomplete(Incomplete),
    /// Requested width out of range for the operation.
    InvalidWidth(InvalidWidth),
}

impl From<Incomplete> for ReadUintError {
    fn from(e: Incomplete) -> Self {
        ReadUintError::Incomplete(e)
    }
}
impl From<InvalidWidth> for ReadUintError {
    fn from(e: InvalidWidth) -> Self {
        ReadUintError::InvalidWidth(e)
    }
}
impl core::fmt::Display for ReadUintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReadUintError::Incomplete(e) => e.fmt(f),
            ReadUintError::InvalidWidth(e) => e.fmt(f),
        }
    }
}
impl core::error::Error for ReadUintError {}

/// Read a variable-width (`0..=16` byte) big-endian unsigned integer into a `u128`.
///
/// The width may come straight off the wire: an out-of-range `n` is a *data*
/// error ([`InvalidWidth`]), not a panic, in every build profile. `n == 0` is
/// legal: it reads nothing and returns `0` (protocols that require a minimum
/// width of 1 must validate that upstream).
///
/// # Errors
/// [`ReadUintError::InvalidWidth`] if `n > 16`;
/// [`ReadUintError::Incomplete`] if fewer than `n` bytes remain.
pub fn read_be_uint(buf: &[u8], n: usize) -> Result<(u128, &[u8]), ReadUintError> {
    if n > 16 {
        return Err(InvalidWidth { max: 16, got: n }.into());
    }
    let (b, rest) = take(buf, n)?;
    let mut acc: u128 = 0;
    for &byte in b {
        acc = (acc << 8) | u128::from(byte);
    }
    Ok((acc, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_splits_and_returns_remainder() {
        let (head, tail) = take(&[1, 2, 3, 4], 2).unwrap();
        assert_eq!(head, &[1, 2]);
        assert_eq!(tail, &[3, 4]);
    }

    #[test]
    fn take_past_end_is_incomplete() {
        let buf = [1, 2, 3];
        assert_eq!(
            take(&buf, 4),
            Err(Incomplete {
                needed: 4,
                available: 3
            })
        );
    }

    #[test]
    fn read_u16_be_reads_big_endian_and_remainder() {
        let (v, rest) = read_u16_be(&[0x12, 0x34, 0x56]).unwrap();
        assert_eq!(v, 0x1234);
        assert_eq!(rest, &[0x56]);
    }

    #[test]
    fn read_u32_be_exact_leaves_empty_remainder() {
        let (v, rest) = read_u32_be(&[0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
        assert_eq!(v, 0xDEAD_BEEF);
        assert!(rest.is_empty());
    }

    #[test]
    fn read_helpers_one_byte_short_are_incomplete() {
        assert_eq!(
            read_u8(&[]),
            Err(Incomplete {
                needed: 1,
                available: 0
            })
        );
        assert_eq!(
            read_u16_be(&[0]),
            Err(Incomplete {
                needed: 2,
                available: 1
            })
        );
        assert_eq!(
            read_u32_be(&[0; 3]),
            Err(Incomplete {
                needed: 4,
                available: 3
            })
        );
        assert_eq!(
            read_u64_be(&[0; 7]),
            Err(Incomplete {
                needed: 8,
                available: 7
            })
        );
    }

    #[test]
    fn read_array_reads_fixed_width() {
        let (arr, rest) = read_array::<3>(&[1, 2, 3, 4]).unwrap();
        assert_eq!(arr, [1, 2, 3]);
        assert_eq!(rest, &[4]);
        assert_eq!(
            read_array::<3>(&[1, 2]),
            Err(Incomplete {
                needed: 3,
                available: 2
            })
        );
    }

    #[test]
    fn read_be_uint_reads_low_n_bytes() {
        let (v, rest) = read_be_uint(&[0x01, 0x02, 0x03], 2).unwrap();
        assert_eq!(v, 0x0102);
        assert_eq!(rest, &[0x03]);
        assert_eq!(
            read_be_uint(&[0x00], 2),
            Err(ReadUintError::Incomplete(Incomplete {
                needed: 2,
                available: 1
            }))
        );
    }

    #[test]
    fn read_be_uint_hostile_width_is_data_error_not_panic() {
        // SE-1: n > 16 must be a recoverable error in ALL build profiles.
        let buf = [0u8; 300];
        assert_eq!(
            read_be_uint(&buf, 255),
            Err(ReadUintError::InvalidWidth(InvalidWidth {
                max: 16,
                got: 255
            }))
        );
        assert_eq!(
            read_be_uint(&buf, 17),
            Err(ReadUintError::InvalidWidth(InvalidWidth {
                max: 16,
                got: 17
            }))
        );
    }

    #[test]
    fn read_be_uint_zero_width_reads_nothing() {
        // Documented contract: n == 0 is legal, reads nothing, returns 0.
        let buf = [0xAA, 0xBB];
        let (v, rest) = read_be_uint(&buf, 0).unwrap();
        assert_eq!(v, 0);
        assert_eq!(rest, &buf);
    }
}
