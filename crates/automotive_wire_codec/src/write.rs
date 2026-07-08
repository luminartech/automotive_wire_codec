//! Big-endian, `core`-only write helpers over [`embedded_io::Write`]. Each returns the
//! number of bytes written. `embedded_io::Write for &mut [u8]` advances the slice and
//! errors (never panics) when the slice is exhausted, so encoding into a too-small
//! stack buffer surfaces as a recoverable `Err`.

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

/// Write the low `n` bytes (`1..=16`) of `value`, big-endian. Returns `n`.
///
/// # Errors
/// The sink's [`embedded_io::ErrorKind`] if the write fails.
///
/// # Panics
/// In debug builds, panics if `n > 16`.
pub fn write_be_uint(
    w: &mut impl Write,
    value: u128,
    n: usize,
) -> Result<usize, embedded_io::ErrorKind> {
    debug_assert!(n <= 16, "write_be_uint: n must be <= 16");
    let bytes = value.to_be_bytes(); // 16 bytes, big-endian
    w.write_all(&bytes[16 - n..]).map_err(|e| e.kind())?;
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
}
