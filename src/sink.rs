//! The [`Sink`] trait and the sinks shipped with the crate.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
