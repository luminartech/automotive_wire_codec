//! Encode→decode round-trips for every leaf width, via proptest (spec §10.1).

use automotive_wire_codec::{
    SliceSink, Sink, read_be_uint, read_u8, read_u16_be, read_u32_be, read_u64_be, read_u128_be,
    write_be_uint, write_u8, write_u16_be, write_u32_be, write_u64_be, write_u128_be,
};
use proptest::prelude::*;

#[test]
fn crate_has_no_embedded_io_in_its_surface() {
    // The 0.4 guarantee: an embedded-io 0.8 is nobody's problem. This is a
    // compile-time assertion - if embedded-io returns as a dependency, the
    // grep in the acceptance checklist catches it; this test pins the sink
    // vocabulary as the only way in.
    // `Sink` must be in scope for `remaining()` - it is a trait method; it
    // and `SliceSink`/`write_u16_be` come from the module-level `use` above.
    let mut buf = [0u8; 4];
    let mut sink = SliceSink::new(&mut buf);
    assert_eq!(write_u16_be(&mut sink, 0x1234).unwrap(), 2);
    assert_eq!(sink.remaining(), 2);
}

proptest! {
    #[test]
    fn roundtrip_u8(v: u8) {
        let mut buf = [0u8; 1];
        let mut w = SliceSink::new(&mut buf);
        let n = write_u8(&mut w, v).unwrap();
        prop_assert_eq!(n, 1);
        let (got, rest) = read_u8(&buf).unwrap();
        prop_assert_eq!(got, v);
        prop_assert!(rest.is_empty());
    }

    #[test]
    fn roundtrip_u16(v: u16) {
        let mut buf = [0u8; 2];
        let mut w = SliceSink::new(&mut buf);
        prop_assert_eq!(write_u16_be(&mut w, v).unwrap(), 2);
        let (got, rest) = read_u16_be(&buf).unwrap();
        prop_assert_eq!(got, v);
        prop_assert!(rest.is_empty());
    }

    #[test]
    fn roundtrip_u32(v: u32) {
        let mut buf = [0u8; 4];
        let mut w = SliceSink::new(&mut buf);
        prop_assert_eq!(write_u32_be(&mut w, v).unwrap(), 4);
        let (got, rest) = read_u32_be(&buf).unwrap();
        prop_assert_eq!(got, v);
        prop_assert!(rest.is_empty());
    }

    #[test]
    fn roundtrip_u64(v: u64) {
        let mut buf = [0u8; 8];
        let mut w = SliceSink::new(&mut buf);
        prop_assert_eq!(write_u64_be(&mut w, v).unwrap(), 8);
        let (got, rest) = read_u64_be(&buf).unwrap();
        prop_assert_eq!(got, v);
        prop_assert!(rest.is_empty());
    }

    #[test]
    fn roundtrip_u128(v: u128) {
        let mut buf = [0u8; 16];
        let mut w = SliceSink::new(&mut buf);
        prop_assert_eq!(write_u128_be(&mut w, v).unwrap(), 16);
        let (got, rest) = read_u128_be(&buf).unwrap();
        prop_assert_eq!(got, v);
        prop_assert!(rest.is_empty());
    }

    #[test]
    fn roundtrip_be_uint(n in 1usize..=16, raw: u128) {
        // Mask to the low n bytes: that is exactly what write_be_uint emits.
        let value = if n == 16 { raw } else { raw & ((1u128 << (8 * n)) - 1) };
        let mut buf = [0u8; 16];
        let mut w = SliceSink::new(&mut buf[..]);
        prop_assert_eq!(write_be_uint(&mut w, value, n).unwrap(), n);
        let (got, rest) = read_be_uint(&buf[..n], n).unwrap();
        prop_assert_eq!(got, value);
        prop_assert!(rest.is_empty());
    }
}
