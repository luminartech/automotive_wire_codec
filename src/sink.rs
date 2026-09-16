//! The [`Sink`] trait and the sinks shipped with the crate.

use crate::error::InsufficientBuffer;
use crate::write::WriteError;

/// A byte sink that knows how much room it has left.
///
/// Implemented by [`SliceSink`](crate::SliceSink),
/// [`CountingSink`](crate::CountingSink) and [`Limited`](crate::Limited). A
/// consumer wrapping a peripheral implements it directly and maps the
/// peripheral's failure to [`WriteError::Io`].
pub trait Sink {
    /// Write every byte, or fail.
    ///
    /// There is no partial write: a codec has no use for one, and a sink that
    /// needs to loop owns that loop internally.
    ///
    /// # Errors
    /// [`WriteError::Insufficient`] if the sink is out of room and knows by how
    /// much; [`WriteError::Io`] if it failed for its own reasons.
    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError>;

    /// Bytes guaranteed writable from here.
    ///
    /// [`usize::MAX`] means no bound is known. A sink that knows its capacity
    /// must report it honestly and must never over-report — a counted overflow
    /// is only possible because this number can be trusted. Implementations
    /// computing it by subtraction must saturate.
    fn remaining(&self) -> usize;
}

/// Encode into a caller-owned buffer, reporting exact capacity.
///
/// The common sink, and what backs
/// [`Encode::encode_to_slice`](crate::Encode::encode_to_slice).
#[derive(Debug)]
pub struct SliceSink<'a> {
    buf: &'a mut [u8],
    written: usize,
}

impl<'a> SliceSink<'a> {
    /// Wrap `buf`, writing from its start.
    #[must_use]
    pub const fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, written: 0 }
    }

    /// Bytes written so far.
    #[must_use]
    pub const fn written(&self) -> usize {
        self.written
    }

    /// Consume the sink and borrow the bytes actually written.
    #[must_use]
    pub fn into_written(self) -> &'a [u8] {
        let n = self.written;
        &self.buf[..n]
    }
}

impl Sink for SliceSink<'_> {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        let end = self.written + buf.len();
        if end > self.buf.len() {
            return Err(InsufficientBuffer {
                needed: end,
                available: self.buf.len(),
            }
            .into());
        }
        self.buf[self.written..end].copy_from_slice(buf);
        self.written = end;
        Ok(())
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::InsufficientBuffer;

    /// A sink that accepts everything and reports no bound.
    struct NullSink;
    impl Sink for NullSink {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), WriteError> {
            Ok(())
        }
        fn remaining(&self) -> usize {
            usize::MAX
        }
    }

    #[test]
    fn unbounded_sink_reports_sentinel() {
        let mut s = NullSink;
        assert!(s.write_all(&[1, 2, 3]).is_ok());
        assert_eq!(s.remaining(), usize::MAX);
    }

    #[test]
    fn slice_sink_writes_and_tracks_remaining() {
        let mut buf = [0u8; 4];
        let mut s = SliceSink::new(&mut buf);
        assert_eq!(s.remaining(), 4);
        s.write_all(&[1, 2]).unwrap();
        assert_eq!(s.written(), 2);
        assert_eq!(s.remaining(), 2);
        s.write_all(&[3]).unwrap();
        assert_eq!(s.into_written(), &[1u8, 2, 3][..]);
    }

    #[test]
    fn slice_sink_overflow_reports_lower_bound_and_capacity() {
        let mut buf = [0u8; 1];
        let mut s = SliceSink::new(&mut buf);
        assert_eq!(
            s.write_all(&[0xAB, 0xCD]),
            Err(WriteError::Insufficient(InsufficientBuffer {
                needed: 2,
                available: 1
            }))
        );
    }

    #[test]
    fn slice_sink_overflow_needed_includes_bytes_already_written() {
        // needed is a lower bound on the total (spec D6): it counts what was
        // already written plus the write that failed, not the whole encode.
        let mut buf = [0u8; 3];
        let mut s = SliceSink::new(&mut buf);
        s.write_all(&[1, 2]).unwrap();
        assert_eq!(
            s.write_all(&[3, 4]),
            Err(WriteError::Insufficient(InsufficientBuffer {
                needed: 4,
                available: 3
            }))
        );
    }

    #[test]
    fn slice_sink_failed_write_is_all_or_nothing() {
        // A rejected write must not leave a partial prefix behind.
        let mut buf = [0xEEu8; 3];
        let mut s = SliceSink::new(&mut buf);
        s.write_all(&[1]).unwrap();
        assert!(s.write_all(&[2, 3, 4]).is_err());
        assert_eq!(s.written(), 1);
        assert_eq!(buf, [1, 0xEE, 0xEE]);
    }

    #[test]
    fn slice_sink_exact_fit_succeeds() {
        let mut buf = [0u8; 2];
        let mut s = SliceSink::new(&mut buf);
        s.write_all(&[1, 2]).unwrap();
        assert_eq!(s.remaining(), 0);
    }

    #[test]
    fn slice_sink_empty_write_always_succeeds() {
        let mut buf = [0u8; 0];
        let mut s = SliceSink::new(&mut buf);
        assert!(s.write_all(&[]).is_ok());
        assert_eq!(s.remaining(), 0);
    }
}
