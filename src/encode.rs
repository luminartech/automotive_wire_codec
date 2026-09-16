//! The [`Encode`] trait: serialize a value into a [`Sink`].

use crate::sink::{CountingSink, Sink, SliceSink};
use crate::write::WriteError;

/// TX-side: serialize `self` into a [`Sink`].
pub trait Encode {
    /// Per-implementation error, constructible from a [`WriteError`] so every
    /// write helper lifts through `?`.
    ///
    /// One `From` impl covers the whole write path — the sink, the fixed-width
    /// helpers, and the variable-width [`write_be_uint`](crate::write_be_uint).
    type Error: From<WriteError>;

    /// Exact number of bytes [`encode`](Encode::encode) will write.
    ///
    /// The default runs `encode` against an infallible [`CountingSink`] and
    /// returns the bytes actually written — correct by construction, so
    /// hand-maintained sizes cannot drift from `encode`. Override only where a
    /// closed-form size is cheaper on a hot path; an override MUST return
    /// exactly the byte count a successful `encode` writes.
    ///
    /// An `encode` implementation that relies on this default must NOT call
    /// `self.encoded_size()` (infinite recursion). Calling `encoded_size()` on
    /// *nested fields* is fine, and is the intended pre-sizing pattern.
    ///
    /// This is no longer on the path to a decent error message — a bounded
    /// sink reports [`WriteError::Insufficient`] directly — but it remains the
    /// answer to "how big is this", and the only way to get an *exact* total
    /// rather than the lower bound a failed write reports.
    ///
    /// # Errors
    /// Whatever `encode` returns for a value that cannot be encoded; the
    /// counting sink itself never fails.
    ///
    /// # Panics
    /// In debug builds, if `encode` returns a byte count different from the
    /// bytes it actually wrote — that is a bug in the `encode` impl
    /// (`written == encoded_size()?` is a hard invariant).
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

    /// Serialize into `sink`; return the number of bytes written.
    ///
    /// **`encode` must be a pure function of `&self`** — same bytes every
    /// call, no observable side effects. The default
    /// [`encoded_size`](Encode::encoded_size) invokes it a second time to
    /// count. An implementation that mutates through interior mutability
    /// (e.g. a rolling sequence counter) will have that side effect applied
    /// per *invocation*, not per frame — advance such state outside `encode`,
    /// then encode the snapshot.
    ///
    /// # Errors
    /// `Self::Error` if the sink rejects a write or the value cannot be encoded.
    fn encode(&self, sink: &mut impl Sink) -> Result<usize, Self::Error>;

    /// Encode into a fixed slice; return the number of bytes written.
    ///
    /// A [`SliceSink`] knows its capacity, so a slice too small fails with
    /// [`WriteError::Insufficient`] carrying `needed`/`available` — lifted
    /// into `Self::Error` like any other write failure. There is no separate
    /// error type and no sizing pass: `encode` runs exactly once whether it
    /// succeeds or fails.
    ///
    /// `needed` is a lower bound, not the encode's total — the encode stopped
    /// at the failing write, so what remained was never measured. Call
    /// [`encoded_size`](Encode::encoded_size) for an exact total.
    ///
    /// On error, `buf` may hold partially written bytes; on success, bytes
    /// past the returned count are untouched.
    ///
    /// # Errors
    /// `Self::Error` if `buf` is too small or the value cannot be encoded.
    fn encode_to_slice(&self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut sink = SliceSink::new(buf);
        self.encode(&mut sink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::InsufficientBuffer;
    use crate::write::{WriteError, write_u16_be};

    #[derive(Debug, Eq, PartialEq)]
    enum TestErr {
        Write(WriteError),
        Value,
    }
    impl From<WriteError> for TestErr {
        fn from(e: WriteError) -> Self {
            TestErr::Write(e)
        }
    }

    struct Val(u16);
    impl Encode for Val {
        type Error = TestErr;
        fn encoded_size(&self) -> Result<usize, TestErr> {
            Ok(2)
        }
        fn encode(&self, sink: &mut impl Sink) -> Result<usize, TestErr> {
            Ok(write_u16_be(sink, self.0)?)
        }
    }

    // Uses the default encoded_size - no hand-written size at all.
    struct TwoVals(u16, u16);
    impl Encode for TwoVals {
        type Error = TestErr;
        fn encode(&self, sink: &mut impl Sink) -> Result<usize, TestErr> {
            let mut n = write_u16_be(sink, self.0)?;
            n += write_u16_be(sink, self.1)?;
            Ok(n)
        }
    }

    // Fails for a VALUE reason: default encoded_size must surface it, not panic.
    struct Rejecting;
    impl Encode for Rejecting {
        type Error = TestErr;
        fn encode(&self, _sink: &mut impl Sink) -> Result<usize, TestErr> {
            Err(TestErr::Value)
        }
    }

    #[test]
    fn encode_writes_and_counts() {
        let mut buf = [0u8; 4];
        let mut sink = SliceSink::new(&mut buf);
        let n = Val(0xABCD).encode(&mut sink).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], &[0xAB, 0xCD]);
    }

    #[test]
    fn default_encoded_size_counts_actual_bytes() {
        assert_eq!(TwoVals(1, 2).encoded_size().unwrap(), 4);
    }

    #[test]
    fn default_encoded_size_surfaces_value_errors() {
        assert_eq!(TwoVals(1, 2).encoded_size().unwrap(), 4);
        assert_eq!(Rejecting.encoded_size(), Err(TestErr::Value));
    }

    #[test]
    fn override_still_supported() {
        assert_eq!(Val(0xABCD).encoded_size().unwrap(), 2);
    }

    #[test]
    fn encode_to_slice_writes_and_counts() {
        let mut buf = [0u8; 4];
        assert_eq!(Val(0xABCD).encode_to_slice(&mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], &[0xAB, 0xCD]);
    }

    #[test]
    fn encode_to_slice_too_small_reports_counts_in_self_error() {
        // One error type on the encode path: no EncodeToSliceError.
        let mut buf = [0u8; 1];
        assert_eq!(
            Val(0xABCD).encode_to_slice(&mut buf),
            Err(TestErr::Write(WriteError::Insufficient(
                InsufficientBuffer {
                    needed: 2,
                    available: 1
                }
            )))
        );
    }

    #[test]
    fn encode_to_slice_propagates_value_errors() {
        let mut buf = [0u8; 8];
        assert_eq!(Rejecting.encode_to_slice(&mut buf), Err(TestErr::Value));
    }

    // Counts encode() invocations; uses the default (counting) encoded_size.
    struct CountsEncodes<'a>(&'a core::cell::Cell<u32>);
    impl Encode for CountsEncodes<'_> {
        type Error = TestErr;
        fn encode(&self, sink: &mut impl Sink) -> Result<usize, TestErr> {
            self.0.set(self.0.get() + 1);
            Ok(write_u16_be(sink, 0xABCD)?)
        }
    }

    #[test]
    fn encode_to_slice_is_single_pass_on_success() {
        let calls = core::cell::Cell::new(0);
        let mut buf = [0u8; 4];
        assert_eq!(CountsEncodes(&calls).encode_to_slice(&mut buf).unwrap(), 2);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn encode_to_slice_is_single_pass_on_failure() {
        // New in 0.4: the classify-after-failure pass is gone, so the failure
        // path costs exactly one encode too.
        let calls = core::cell::Cell::new(0);
        let mut buf = [0u8; 1];
        assert!(CountsEncodes(&calls).encode_to_slice(&mut buf).is_err());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn counting_sink_backs_encode() {
        let mut sink = CountingSink::new();
        assert_eq!(Val(0xABCD).encode(&mut sink).unwrap(), 2);
        assert_eq!(sink.count(), 2);
        Val(0x0102).encode(&mut sink).unwrap();
        assert_eq!(sink.count(), 4);
    }
}
