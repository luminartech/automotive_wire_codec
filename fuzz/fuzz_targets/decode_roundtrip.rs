#![no_main]

use automotive_wire_codec::{
    read_array, read_be_uint, read_u16_be, read_u32_be, read_u64_be, read_u8, take,
    write_be_uint, write_u16_be,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // No-panic: leaf reads over arbitrary input must never panic (errors are fine).
    let _ = take(data, data.len().min(3));
    let _ = read_u8(data);
    let _ = read_u16_be(data);
    let _ = read_u32_be(data);
    let _ = read_u64_be(data);
    let _ = read_array::<4>(data);
    if let Some(&first) = data.first() {
        // Any width, including hostile > 16: must return Err, never panic.
        let _ = read_be_uint(data, usize::from(first));
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

    // write_be_uint must never panic for ANY n — hostile widths return Err.
    let mut wide = [0u8; 16];
    let mut ww: &mut [u8] = &mut wide;
    let _ = write_be_uint(&mut ww, u128::from(v), 2);
    let mut ww2: &mut [u8] = &mut wide;
    let _ = write_be_uint(&mut ww2, u128::from(v), usize::from(data.first().copied().unwrap_or(0)));
});
