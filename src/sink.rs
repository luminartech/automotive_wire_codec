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

/// A mutable reference to a sink is itself a sink.
///
/// Mirrors `embedded_io::Write for &mut W` and `std::io::Write for &mut W`.
/// Without this, a function holding `sink: &mut impl Sink` could not wrap its
/// own sink (`Limited::new(&mut *sink, n)` would not compile), and
/// `&mut dyn Sink` could never be passed where `impl Sink` is wanted, since
/// `impl Sink` implies `Sized`.
impl<S: Sink + ?Sized> Sink for &mut S {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        (**self).write_all(buf)
    }

    fn remaining(&self) -> usize {
        (**self).remaining()
    }
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
        self.buf.len().saturating_sub(self.written)
    }
}

/// Sink that counts bytes and stores nothing.
///
/// Backs the default [`Encode::encoded_size`](crate::Encode::encoded_size);
/// also useful in consumer tests to assert an `encoded_size` override agrees
/// with `encode`. Never fails.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

impl Sink for CountingSink {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        self.count += buf.len();
        Ok(())
    }

    fn remaining(&self) -> usize {
        usize::MAX
    }
}

/// Bound any sink to a byte budget.
///
/// What a transport driver wraps around its response buffer to enforce an
/// advertised maximum: an over-long encode then fails at the write with counts
/// attached, instead of needing a pre-sizing pass.
#[derive(Clone, Debug)]
pub struct Limited<S> {
    inner: S,
    budget: usize,
    written: usize,
}

impl<S: Sink> Limited<S> {
    /// Bound `inner` to `budget` bytes.
    #[must_use]
    pub const fn new(inner: S, budget: usize) -> Self {
        Self {
            inner,
            budget,
            written: 0,
        }
    }
}

impl<S: Sink> Sink for Limited<S> {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        let end = self.written + buf.len();
        if end > self.budget {
            return Err(InsufficientBuffer {
                needed: end,
                available: self.budget,
            }
            .into());
        }
        self.inner.write_all(buf)?;
        self.written = end;
        Ok(())
    }

    fn remaining(&self) -> usize {
        self.budget
            .saturating_sub(self.written)
            .min(self.inner.remaining())
    }
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

    #[test]
    fn counting_sink_counts_and_reports_no_bound() {
        let mut s = CountingSink::new();
        assert_eq!(s.remaining(), usize::MAX);
        s.write_all(&[1, 2, 3]).unwrap();
        s.write_all(&[4]).unwrap();
        assert_eq!(s.count(), 4);
        assert_eq!(s.remaining(), usize::MAX);
    }

    #[test]
    fn limited_enforces_budget_below_inner_capacity() {
        // The 0x14 story: a driver bounds a large buffer to the transport's
        // advertised maximum, and the overflow is counted against the budget.
        let mut buf = [0u8; 64];
        let mut s = Limited::new(SliceSink::new(&mut buf), 4);
        assert_eq!(s.remaining(), 4);
        s.write_all(&[1, 2, 3]).unwrap();
        assert_eq!(s.remaining(), 1);
        assert_eq!(
            s.write_all(&[4, 5]),
            Err(WriteError::Insufficient(InsufficientBuffer {
                needed: 5,
                available: 4
            }))
        );
    }

    #[test]
    fn limited_rejects_without_writing_through() {
        // A write refused by the budget must not reach the inner sink.
        let mut buf = [0xEEu8; 64];
        {
            let mut s = Limited::new(SliceSink::new(&mut buf), 2);
            assert!(s.write_all(&[1, 2, 3]).is_err());
        }
        assert_eq!(buf[0], 0xEE);
    }

    #[test]
    fn limited_remaining_is_min_of_budget_and_inner() {
        // Budget larger than the inner sink: the inner bound wins.
        let mut buf = [0u8; 3];
        let s = Limited::new(SliceSink::new(&mut buf), 100);
        assert_eq!(s.remaining(), 3);
    }

    #[test]
    fn limited_over_unbounded_inner_reports_budget() {
        let s = Limited::new(CountingSink::new(), 7);
        assert_eq!(s.remaining(), 7);
    }

    #[test]
    fn limited_zero_budget_rejects_any_nonempty_write() {
        let mut s = Limited::new(CountingSink::new(), 0);
        assert_eq!(s.remaining(), 0);
        assert!(s.write_all(&[1]).is_err());
        assert!(s.write_all(&[]).is_ok());
    }

    /// Mirrors `Encode::encode(&self, sink: &mut impl Sink)`: `impl Sink`
    /// implies `Sized`, so this only accepts a *sized* sink — including
    /// `&mut dyn Sink`, which is a thin, sized pointer type in its own right.
    fn write_via_generic_sink(sink: &mut impl Sink, buf: &[u8]) -> Result<(), WriteError> {
        sink.write_all(buf)
    }

    #[test]
    fn mut_ref_to_slice_sink_is_itself_a_sink() {
        let mut buf = [0u8; 4];
        let mut inner = SliceSink::new(&mut buf);
        let mut r = &mut inner;
        write_via_generic_sink(&mut r, &[1, 2]).unwrap();
        assert_eq!(r.remaining(), 2);
        assert_eq!(inner.remaining(), 2);
        assert_eq!(inner.written(), 2);
    }

    #[test]
    fn limited_can_wrap_a_borrowed_sink() {
        // The motivating case: a function holding `sink: &mut impl Sink`
        // wraps its own borrow in `Limited` without taking ownership.
        fn encode_bounded(sink: &mut impl Sink, budget: usize) -> Result<(), WriteError> {
            let mut limited = Limited::new(&mut *sink, budget);
            limited.write_all(&[1, 2, 3, 4, 5])
        }

        let mut buf = [0u8; 64];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(
            encode_bounded(&mut sink, 3),
            Err(WriteError::Insufficient(InsufficientBuffer {
                needed: 5,
                available: 3
            }))
        );
        // The budget rejected it before it ever reached the inner sink.
        assert_eq!(sink.written(), 0);

        assert!(encode_bounded(&mut sink, 10).is_ok());
        assert_eq!(sink.written(), 5);
    }

    #[test]
    fn mut_dyn_sink_accepted_where_sink_is_wanted() {
        // A caller holding `&mut dyn Sink` (e.g. behind dynamic dispatch)
        // reborrows it into a function wanting `&mut impl Sink`. `impl Sink`
        // implies `Sized`; `&mut dyn Sink` itself is a sized, thin pointer,
        // so this only compiles because `&mut dyn Sink: Sink` via the
        // blanket `impl<S: Sink + ?Sized> Sink for &mut S`.
        let mut buf = [0u8; 4];
        let mut inner = SliceSink::new(&mut buf);
        let mut dyn_sink: &mut dyn Sink = &mut inner;
        write_via_generic_sink(&mut dyn_sink, &[9, 8]).unwrap();
        assert_eq!(inner.written(), 2);
    }

    #[test]
    fn limited_inner_refusal_after_budget_pass_leaves_written_unadvanced() {
        // The budget allows the write, but the inner sink cannot take it.
        // `needed`/`available` must be the inner sink's, not the budget's,
        // and `Limited::written` must not advance on the inner failure.
        let mut buf = [0u8; 3];
        let mut s = Limited::new(SliceSink::new(&mut buf), 100);
        assert_eq!(
            s.write_all(&[1, 2, 3, 4, 5]),
            Err(WriteError::Insufficient(InsufficientBuffer {
                needed: 5,
                available: 3
            }))
        );
        // Budget still reports as if nothing was written.
        assert_eq!(s.remaining(), 3);
        // A subsequent write within the inner sink's real capacity succeeds,
        // proving `written` was not advanced by the failed one.
        assert!(s.write_all(&[1, 2, 3]).is_ok());
    }
}
