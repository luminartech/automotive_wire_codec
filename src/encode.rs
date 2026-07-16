//! The [`Encode`] trait: serialize a value into an [`embedded_io::Write`] sink.

use embedded_io::Write;

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
    /// MUST equal the byte count returned by a successful `encode`. This is a hard
    /// invariant — nested encoders rely on it to reserve space without a staging buffer.
    fn encoded_size(&self) -> usize;

    /// Serialize into `writer`; return the number of bytes written.
    ///
    /// # Errors
    /// `Self::Error` if the sink rejects a write.
    fn encode(&self, writer: &mut impl Write) -> Result<usize, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
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
        fn encoded_size(&self) -> usize {
            2
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
        assert_eq!(n, v.encoded_size());
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
}
