#![no_main]

use automotive_wire_codec::{
    read_array, read_be_uint, read_be_uint_into, read_optional_array, read_u8, read_u16_be,
    read_u32_be, read_u64_be, read_u128_be, take, write_be_uint, write_u16_be, write_u128_be,
    DecodeIter, Encode, EncodeToSliceError, Incomplete, InsufficientBuffer, ensure_len,
    minimal_be_len,
};
use libfuzzer_sys::fuzz_target;

// A 3-byte fixed-stride record, mirroring the documented DecodeIter shape
// (clean end BEFORE decode; partial tail is a real error).
struct Rec(#[allow(dead_code)] [u8; 3]);
impl<'a> DecodeIter<'a> for Rec {
    type Error = Incomplete;
    const WIRE_SIZE: Option<usize> = Some(3);
    fn decode_next(buf: &'a [u8]) -> Result<Option<(Self, &'a [u8])>, Incomplete> {
        if buf.is_empty() {
            return Ok(None);
        }
        let (b, rest) = read_array::<3>(buf)?;
        Ok(Some((Rec(b), rest)))
    }
}

// A 2-byte value relying on the DEFAULT (counting) encoded_size, so the
// fuzzer exercises CountingSink and encode_to_slice's error classification.
struct V(u16);
impl Encode for V {
    type Error = embedded_io::ErrorKind;
    fn encode(&self, w: &mut impl embedded_io::Write) -> Result<usize, Self::Error> {
        write_u16_be(w, self.0)
    }
}

fuzz_target!(|data: &[u8]| {
    // No-panic: leaf reads over arbitrary input must never panic (errors are fine).
    let _ = take(data, data.len().min(3));
    let _ = read_u8(data);
    let _ = read_u16_be(data);
    let _ = read_u32_be(data);
    let _ = read_u64_be(data);
    let _ = read_u128_be(data);
    let _ = read_array::<4>(data);
    let _ = read_optional_array::<4>(data);
    let _ = ensure_len(data, data.len());
    let _ = ensure_len(data, data.len().wrapping_add(1));

    if let Some(&first) = data.first() {
        // Any width, including hostile > 16: must return Err, never panic.
        let _ = read_be_uint(data, usize::from(first));
        let _ = read_be_uint_into::<u32>(data, usize::from(first));

        // A guaranteed-VALID width (0..=16) on every input, so the
        // take/accumulate path is always exercised, not just the n > 16 guard.
        let n = usize::from(first) % 17;
        if let Ok((value, rest)) = read_be_uint(data, n) {
            assert_eq!(rest.len(), data.len() - n);
            // Read/write round-trip at the same width: writing the value
            // back with width n reproduces the n input bytes exactly.
            let mut buf = [0u8; 16];
            let mut w: &mut [u8] = &mut buf;
            assert_eq!(write_be_uint(&mut w, value, n).unwrap(), n);
            assert_eq!(&buf[..n], &data[..n]);
            // Minimal-width round-trip: minimal_be_len never over-reports,
            // and loses nothing.
            let m = minimal_be_len(value);
            assert!(m <= n);
            let mut buf2 = [0u8; 16];
            let mut w2: &mut [u8] = &mut buf2;
            assert_eq!(write_be_uint(&mut w2, value, m).unwrap(), m);
            assert_eq!(read_be_uint(&buf2[..m], m).unwrap().0, value);
        }
        // Typed variant at a valid u32 width round-trips through u128.
        let n4 = usize::from(first) % 5;
        if let Ok((v32, _)) = read_be_uint_into::<u32>(data, n4) {
            assert_eq!(read_be_uint(data, n4).unwrap().0, u128::from(v32));
        }
    }

    // Round-trip: encode a u16 derived from the input and decode it back.
    let v = u16::from_le_bytes([
        data.first().copied().unwrap_or(0),
        data.get(1).copied().unwrap_or(0),
    ]);
    let mut buf = [0u8; 2];
    let mut w: &mut [u8] = &mut buf;
    assert_eq!(write_u16_be(&mut w, v).unwrap(), 2);
    let (decoded, rest) = read_u16_be(&buf).unwrap();
    assert_eq!(decoded, v);
    assert!(rest.is_empty());

    // u128 round-trip through the widest fixed helpers.
    let v128 = u128::from(v) << 112 | u128::from(v);
    let mut buf16 = [0u8; 16];
    let mut w16: &mut [u8] = &mut buf16;
    assert_eq!(write_u128_be(&mut w16, v128).unwrap(), 16);
    assert_eq!(read_u128_be(&buf16).unwrap().0, v128);

    // write_be_uint must never panic for ANY n — hostile widths return Err.
    let mut wide = [0u8; 16];
    let mut ww: &mut [u8] = &mut wide;
    let _ = write_be_uint(&mut ww, u128::from(v), 2);
    let mut ww2: &mut [u8] = &mut wide;
    let _ = write_be_uint(&mut ww2, u128::from(v), usize::from(data.first().copied().unwrap_or(0)));

    // encode_to_slice classification, via the default (counting) encoded_size:
    // an exact buffer succeeds single-pass; a short one reports both counts.
    let mut exact = [0u8; 2];
    assert_eq!(V(v).encode_to_slice(&mut exact).unwrap(), 2);
    assert_eq!(exact, buf);
    let mut short = [0u8; 1];
    assert!(matches!(
        V(v).encode_to_slice(&mut short),
        Err(EncodeToSliceError::InsufficientBuffer(InsufficientBuffer {
            needed: 2,
            available: 1,
        }))
    ));

    // DecodeIterator invariant: remaining_len counts exactly the items next()
    // will yield (a partial tail counts as its one Err item), and reports
    // Some(0) only once fused.
    let mut it = Rec::iter(data);
    let predicted = it.remaining_len().expect("fixed stride");
    assert_eq!(predicted, data.len().div_ceil(3));
    let mut yielded = 0usize;
    let mut saw_err = false;
    for item in it.by_ref() {
        yielded += 1;
        if item.is_err() {
            saw_err = true;
        }
    }
    assert_eq!(yielded, predicted);
    assert_eq!(saw_err, data.len() % 3 != 0);
    assert_eq!(it.remaining_len(), Some(0));
});
