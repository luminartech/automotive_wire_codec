//! Big-endian, `core`-only write helpers over [`embedded_io::Write`]. Each returns the
//! number of bytes written. `embedded_io::Write for &mut [u8]` advances the slice and
//! errors (never panics) when the slice is exhausted, so encoding into a too-small
//! stack buffer surfaces as a recoverable `Err` — specifically
//! [`embedded_io::ErrorKind::WriteZero`], which carries no needed/available counts
//! (a generic sink cannot know its capacity). For counted diagnostics encode via
//! [`Encode::encode_to_slice`](crate::Encode::encode_to_slice), which classifies
//! a failed encode against [`Encode::encoded_size`](crate::Encode::encoded_size)
//! and returns [`InsufficientBuffer`](crate::InsufficientBuffer).

use crate::error::InvalidWidth;
use embedded_io::{Error, Write};

/// Write a single byte. Returns `1`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_u8(w: &mut impl Write, v: u8) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(&[v]).map_err(|e| e.kind())?;
    Ok(1)
}

/// Write a big-endian `u16`. Returns `2`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_u16_be(w: &mut impl Write, v: u16) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(&v.to_be_bytes()).map_err(|e| e.kind())?;
    Ok(2)
}

/// Write a big-endian `u32`. Returns `4`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_u32_be(w: &mut impl Write, v: u32) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(&v.to_be_bytes()).map_err(|e| e.kind())?;
    Ok(4)
}

/// Write a big-endian `u64`. Returns `8`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_u64_be(w: &mut impl Write, v: u64) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(&v.to_be_bytes()).map_err(|e| e.kind())?;
    Ok(8)
}

/// Write a big-endian `u128`. Returns `16`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_u128_be(w: &mut impl Write, v: u128) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(&v.to_be_bytes()).map_err(|e| e.kind())?;
    Ok(16)
}

/// Error from the variable-width write helper ([`write_be_uint`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteUintError {
    /// The sink rejected a write.
    Io(embedded_io::ErrorKind),
    /// Requested width out of range for the operation.
    InvalidWidth(InvalidWidth),
}

impl From<embedded_io::ErrorKind> for WriteUintError {
    fn from(e: embedded_io::ErrorKind) -> Self {
        WriteUintError::Io(e)
    }
}
impl From<InvalidWidth> for WriteUintError {
    fn from(e: InvalidWidth) -> Self {
        WriteUintError::InvalidWidth(e)
    }
}
impl core::fmt::Display for WriteUintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WriteUintError::Io(kind) => write!(f, "write failed: {kind:?}"),
            WriteUintError::InvalidWidth(e) => e.fmt(f),
        }
    }
}
impl core::error::Error for WriteUintError {}

/// Minimal number of big-endian bytes needed to represent `value` — computes
/// the width to pass to [`write_be_uint`] for protocols that emit
/// minimal-width length/size/address fields.
///
/// `minimal_be_len(0) == 0`; protocols that require at least one byte apply
/// `.max(1)`. The result is always `<= 16`, so it is a valid width for
/// [`write_be_uint`], and it is `usize` so
/// `write_be_uint(w, v, minimal_be_len(v))` needs no cast at the call site.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // result is 0..=16
pub const fn minimal_be_len(value: u128) -> usize {
    (u128::BITS - value.leading_zeros()).div_ceil(8) as usize
}

/// Write the low `n` bytes (`0..=16`) of `value`, big-endian. Returns `n`.
///
/// The width may come straight off the wire: an out-of-range `n` is a *data*
/// error ([`InvalidWidth`]), not a panic, in every build profile. `n == 0` is
/// legal and writes nothing.
///
/// **This helper does not check that `value` fits in `n` bytes.** Bytes of
/// `value` above the low `n` are silently dropped: if
/// `minimal_be_len(value) > n`, the emitted field decodes to a *different*
/// value (e.g. `write_be_uint(w, 0x1_0000, 2)` writes `[0x00, 0x00]`, which
/// reads back as `0`). Callers emitting a field whose value can grow — length
/// prefixes especially — must validate `minimal_be_len(value) <= n` before
/// writing, or compute the width with [`minimal_be_len`] so nothing is lost.
///
/// # Errors
/// [`WriteUintError::InvalidWidth`] if `n > 16`; [`WriteUintError::Io`] if the
/// sink rejects the write.
pub fn write_be_uint(w: &mut impl Write, value: u128, n: usize) -> Result<usize, WriteUintError> {
    if n > 16 {
        return Err(InvalidWidth { max: 16, got: n }.into());
    }
    let bytes = value.to_be_bytes(); // 16 bytes, big-endian
    w.write_all(&bytes[16 - n..])
        .map_err(|e| WriteUintError::from(e.kind()))?;
    Ok(n)
}

/// Write a raw byte slice verbatim (e.g. an opaque payload). Returns `bytes.len()`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
pub fn write_all(w: &mut impl Write, bytes: &[u8]) -> Result<usize, embedded_io::ErrorKind> {
    w.write_all(bytes).map_err(|e| e.kind())?;
    Ok(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::InvalidWidth;

    #[test]
    fn write_u16_be_writes_big_endian_and_counts() {
        let mut buf = [0u8; 4];
        let mut w: &mut [u8] = &mut buf;
        let n = write_u16_be(&mut w, 0x1234).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], &[0x12, 0x34]);
    }

    #[test]
    fn write_be_uint_writes_only_low_n_bytes() {
        let mut buf = [0u8; 3];
        let mut w: &mut [u8] = &mut buf;
        // value has high bytes set; only the low 3 must be written, big-endian.
        let n = write_be_uint(&mut w, 0xAABB_CCDD_u128, 3).unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf, [0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn write_all_writes_verbatim() {
        let mut buf = [0u8; 4];
        let mut w: &mut [u8] = &mut buf;
        let n = write_all(&mut w, &[9, 8, 7]).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[9, 8, 7]);
    }

    #[test]
    fn write_into_too_small_slice_errors_not_panics() {
        let mut buf = [0u8; 1];
        let mut w: &mut [u8] = &mut buf;
        assert!(write_u16_be(&mut w, 0x1234).is_err());
    }

    #[test]
    fn write_be_uint_hostile_width_is_data_error_not_panic() {
        // SE-1 write half: previously `16 - n` underflowed and PANICKED in
        // release builds. Must now be a recoverable error in all profiles.
        let mut buf = [0u8; 300];
        let mut w: &mut [u8] = &mut buf;
        assert_eq!(
            write_be_uint(&mut w, 0xABCD, 255),
            Err(WriteUintError::InvalidWidth(InvalidWidth {
                max: 16,
                got: 255
            }))
        );
    }

    #[test]
    fn write_be_uint_zero_width_writes_nothing() {
        let mut buf = [0xFFu8; 2];
        let mut w: &mut [u8] = &mut buf;
        assert_eq!(write_be_uint(&mut w, 0xABCD, 0).unwrap(), 0);
        assert_eq!(buf, [0xFF, 0xFF]);
    }

    #[test]
    fn write_u128_be_writes_big_endian_and_counts() {
        let v = 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10_u128;
        let mut buf = [0u8; 16];
        let mut w: &mut [u8] = &mut buf;
        assert_eq!(write_u128_be(&mut w, v).unwrap(), 16);
        assert_eq!(buf, v.to_be_bytes());
    }

    #[test]
    fn minimal_be_len_boundaries() {
        // uds P3/SE-3 contract: 0 needs 0 bytes; callers wanting >=1 use .max(1).
        assert_eq!(minimal_be_len(0), 0);
        assert_eq!(minimal_be_len(1), 1);
        assert_eq!(minimal_be_len(0xFF), 1);
        assert_eq!(minimal_be_len(0x100), 2);
        assert_eq!(minimal_be_len(0xFFFF), 2);
        assert_eq!(minimal_be_len(0x1_0000), 3);
        assert_eq!(minimal_be_len(u128::from(u64::MAX)), 8);
        assert_eq!(minimal_be_len(u128::MAX), 16);
    }

    #[test]
    fn write_be_uint_truncates_value_wider_than_n() {
        // Characterization of the documented contract: the helper does NOT
        // check minimality — an over-wide value is silently truncated to its
        // low n bytes and round-trips to a different value. Callers guard
        // with minimal_be_len(value) <= n.
        let mut buf = [0xEEu8; 4];
        let mut w: &mut [u8] = &mut buf;
        assert_eq!(write_be_uint(&mut w, 0x1_0000, 2).unwrap(), 2);
        assert_eq!(&buf[..2], &[0x00, 0x00]); // high byte dropped, reads back as 0
    }

    #[test]
    fn minimal_be_len_pairs_with_write_be_uint() {
        // The minimal width loses nothing on a write/read round trip, and the
        // advertised idiom needs no cast at the call site.
        let v = 0x00AB_CDEF_u128;
        let n = minimal_be_len(v);
        assert_eq!(n, 3);
        let mut buf = [0u8; 16];
        let mut w: &mut [u8] = &mut buf;
        assert_eq!(write_be_uint(&mut w, v, minimal_be_len(v)).unwrap(), n);
        assert_eq!(&buf[..n], &[0xAB, 0xCD, 0xEF]);
    }
}
