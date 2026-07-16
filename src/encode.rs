//! The [`Encode`] trait: serialize a value into an [`embedded_io::Write`] sink.

use embedded_io::Write;

use crate::error::InsufficientBuffer;

/// Error from [`Encode::encode_to_slice`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeToSliceError<E> {
    /// The slice was smaller than [`Encode::encoded_size`]; carries both counts.
    InsufficientBuffer(InsufficientBuffer),
    /// The value itself failed to encode (or to size).
    Encode(E),
}

impl<E> From<InsufficientBuffer> for EncodeToSliceError<E> {
    fn from(e: InsufficientBuffer) -> Self {
        EncodeToSliceError::InsufficientBuffer(e)
    }
}
impl<E: core::fmt::Display> core::fmt::Display for EncodeToSliceError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EncodeToSliceError::InsufficientBuffer(e) => e.fmt(f),
            EncodeToSliceError::Encode(e) => e.fmt(f),
        }
    }
}
impl<E: core::fmt::Debug + core::fmt::Display> core::error::Error for EncodeToSliceError<E> {}

/// Infallible [`embedded_io::Write`] sink that counts bytes and stores nothing.
///
/// Backs the default [`Encode::encoded_size`]; also useful in consumer tests to
/// assert an `encoded_size` override agrees with `encode`.
#[derive(Debug, Default)]
pub struct CountingSink {
    count: usize,
}

impl CountingSink {
    /// New sink with a zero count.
    #[must_use]
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    /// Total bytes written so far.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}

impl embedded_io::ErrorType for CountingSink {
    type Error = core::convert::Infallible;
}

impl Write for CountingSink {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.count += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// TX-side: serialize `self` into an [`embedded_io::Write`] sink.
pub trait Encode {
    /// Per-implementation error; constructible from an I/O [`embedded_io::ErrorKind`]
    /// so the `write_*` leaf helpers lift through `?`.
    type Error: From<embedded_io::ErrorKind>;

    /// Exact number of bytes [`encode`](Encode::encode) will write.
    ///
    /// The default runs `encode` against an infallible [`CountingSink`] and
    /// returns the bytes actually written — correct by construction, so
    /// hand-maintained sizes cannot drift from `encode` (the bug class every
    /// migrated consumer had). Override only where a closed-form size is
    /// cheaper on a hot path; an override MUST return exactly the byte count a
    /// successful `encode` writes — nested encoders reserve space from it with
    /// no staging buffer.
    ///
    /// An `encode` implementation that relies on this default must NOT call
    /// `self.encoded_size()` (infinite recursion). Calling `encoded_size()` on
    /// *nested fields* is fine, and is the intended pre-sizing pattern.
    ///
    /// # Errors
    /// Whatever `encode` returns for a value that cannot be encoded; the
    /// counting sink itself never fails.
    fn encoded_size(&self) -> Result<usize, Self::Error> {
        let mut sink = CountingSink::new();
        let reported = self.encode(&mut sink)?;
        debug_assert!(
            reported == sink.count(),
            "encode returned {reported} but wrote {} bytes",
            sink.count()
        );
        Ok(sink.count())
    }

    /// Serialize into `writer`; return the number of bytes written.
    ///
    /// # Errors
    /// `Self::Error` if the sink rejects a write or the value cannot be encoded.
    fn encode(&self, writer: &mut impl Write) -> Result<usize, Self::Error>;

    /// Encode into a fixed slice, pre-checking capacity against
    /// [`encoded_size`](Encode::encoded_size) so a too-small buffer reports
    /// `needed`/`available` ([`InsufficientBuffer`]) instead of a bare
    /// [`embedded_io::ErrorKind::WriteZero`], and hiding the
    /// `&mut &mut [u8]` cursor re-borrow every fixed-buffer call site
    /// otherwise writes by hand.
    ///
    /// # Errors
    /// [`EncodeToSliceError::InsufficientBuffer`] if `buf` is smaller than
    /// `encoded_size()`; [`EncodeToSliceError::Encode`] if sizing or encoding
    /// itself fails.
    fn encode_to_slice(&self, buf: &mut [u8]) -> Result<usize, EncodeToSliceError<Self::Error>> {
        let needed = self.encoded_size().map_err(EncodeToSliceError::Encode)?;
        if buf.len() < needed {
            return Err(InsufficientBuffer {
                needed,
                available: buf.len(),
            }
            .into());
        }
        let mut cursor: &mut [u8] = buf;
        self.encode(&mut cursor).map_err(EncodeToSliceError::Encode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::InsufficientBuffer;
    use crate::write::write_u16_be;

    #[derive(Debug)]
    enum TestErr {
        Io(embedded_io::ErrorKind),
    }
    impl From<embedded_io::ErrorKind> for TestErr {
        fn from(kind: embedded_io::ErrorKind) -> Self {
            TestErr::Io(kind)
        }
    }

    struct Val(u16);
    impl Encode for Val {
        type Error = TestErr;
        fn encoded_size(&self) -> Result<usize, TestErr> {
            Ok(2)
        }
        fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, TestErr> {
            Ok(write_u16_be(writer, self.0)?)
        }
    }

    #[test]
    fn encode_reports_size_and_writes_into_slice() {
        let v = Val(0xABCD);
        let mut buf = [0u8; 4];
        let mut w: &mut [u8] = &mut buf;
        let n = v.encode(&mut w).unwrap();
        assert_eq!(n, v.encoded_size().unwrap());
        assert_eq!(&buf[..2], &[0xAB, 0xCD]);
    }

    #[test]
    fn encode_into_too_small_slice_errors() {
        let v = Val(0xABCD);
        let mut buf = [0u8; 1];
        let mut w: &mut [u8] = &mut buf;
        let err = v.encode(&mut w).unwrap_err();
        // Reads the `Io` field so it is load-bearing (irrefutable: single-variant enum).
        let TestErr::Io(kind) = err;
        // `embedded_io::Write for &mut [u8]` yields `SliceWriteError::Full`, whose
        // `kind()` is `WriteZero`, when the sink is exhausted mid-write.
        assert_eq!(kind, embedded_io::ErrorKind::WriteZero);
    }

    #[test]
    fn counting_sink_counts_and_never_fails() {
        let mut sink = CountingSink::new();
        // Uses the existing test type Val(u16) which writes 2 bytes.
        let n = Val(0xABCD).encode(&mut sink).unwrap();
        assert_eq!(n, 2);
        assert_eq!(sink.count(), 2);
        // Accumulates across encodes.
        Val(0x0102).encode(&mut sink).unwrap();
        assert_eq!(sink.count(), 4);
    }

    // Uses the default encoded_size — no hand-written size at all.
    struct TwoVals(u16, u16);
    impl Encode for TwoVals {
        type Error = TestErr;
        fn encode(&self, writer: &mut impl embedded_io::Write) -> Result<usize, TestErr> {
            let mut n = write_u16_be(writer, self.0)?;
            n += write_u16_be(writer, self.1)?;
            Ok(n)
        }
    }

    // Encode fails for a VALUE reason (uds C1 shape): default encoded_size
    // must surface it as Err, not panic.
    struct Rejecting;
    impl Encode for Rejecting {
        type Error = TestErr;
        fn encode(&self, _writer: &mut impl embedded_io::Write) -> Result<usize, TestErr> {
            Err(TestErr::Io(embedded_io::ErrorKind::InvalidData))
        }
    }

    #[test]
    fn default_encoded_size_counts_actual_bytes() {
        assert_eq!(TwoVals(1, 2).encoded_size().unwrap(), 4);
    }

    #[test]
    fn default_encoded_size_surfaces_value_errors() {
        assert!(Rejecting.encoded_size().is_err());
    }

    #[test]
    fn override_still_supported() {
        // Val overrides encoded_size with a closed form (see impl above).
        assert_eq!(Val(0xABCD).encoded_size().unwrap(), 2);
    }

    #[test]
    fn encode_to_slice_writes_and_counts() {
        // SE-5: no `let mut w: &mut [u8] = &mut buf;` dance at the call site.
        let mut buf = [0u8; 4];
        let n = Val(0xABCD).encode_to_slice(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], &[0xAB, 0xCD]);
    }

    #[test]
    fn encode_to_slice_too_small_reports_both_counts() {
        // someip F5: needed/available diagnostics, not a bare WriteZero.
        let mut buf = [0u8; 1];
        let err = Val(0xABCD).encode_to_slice(&mut buf).unwrap_err();
        assert!(matches!(
            err,
            EncodeToSliceError::InsufficientBuffer(InsufficientBuffer {
                needed: 2,
                available: 1
            })
        ));
    }

    #[test]
    fn encode_to_slice_propagates_encode_errors() {
        let mut buf = [0u8; 8];
        let err = Rejecting.encode_to_slice(&mut buf).unwrap_err();
        assert!(matches!(err, EncodeToSliceError::Encode(_)));
    }
}
