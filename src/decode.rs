//! RX-side traits: [`Decode`] (zero-copy decode borrowing from the buffer),
//! [`DecodeIter`] (repeated same-typed elements), and the [`DecodeIterator`] adapter.

use core::marker::PhantomData;

use crate::error::{Incomplete, TrailingBytes};

/// RX-side: zero-copy decode borrowing from `buf`. The value is valid only as long as
/// `buf` lives.
pub trait Decode<'a>: Sized {
    /// Per-implementation error; constructible from [`Incomplete`] and [`TrailingBytes`]
    /// so the leaf read helpers and the `decode_exact` default lift through `?`.
    type Error: From<Incomplete> + From<TrailingBytes>;

    /// Decode from the FRONT of `buf`; return `(value, unconsumed_remainder)`.
    ///
    /// # Errors
    /// `Self::Error` if the input is malformed or too short.
    fn decode(buf: &'a [u8]) -> Result<(Self, &'a [u8]), Self::Error>;

    /// Decode requiring the ENTIRE buffer to be consumed. Use this at a message
    /// boundary where the length is already known.
    ///
    /// Do NOT use it where a buffer may legally hold more than one message
    /// (e.g. a multi-message datagram): there, trailing bytes are the *next*
    /// message, not an error — use [`decode`](Decode::decode) and thread the
    /// remainder.
    ///
    /// # Errors
    /// [`TrailingBytes`] (via `Self::Error`) if bytes remain, plus anything
    /// [`decode`](Decode::decode) can return.
    fn decode_exact(buf: &'a [u8]) -> Result<Self, Self::Error> {
        let (value, rest) = Self::decode(buf)?;
        if rest.is_empty() {
            Ok(value)
        } else {
            Err(TrailingBytes(rest.len()).into())
        }
    }
}

/// RX-side: decode a sequence of same-typed elements from a buffer. Implement this only
/// for protocols that have repeated elements (e.g. a UDS DTC record list).
pub trait DecodeIter<'a>: Sized {
    /// Per-implementation error; constructible from [`Incomplete`].
    type Error: From<Incomplete>;

    /// Wire size of one element, when fixed at compile time (must be non-zero
    /// if `Some`). Enables [`DecodeIterator::remaining_len`] for fixed-stride
    /// record streams. Default: `None` (variable-width).
    const WIRE_SIZE: Option<usize> = None;

    /// Decode the next element from the front of `buf`.
    ///
    /// Returns `Ok(Some((value, rest)))` for an element, `Ok(None)` for a clean end
    /// (buffer empty / no more elements), or `Err(_)` for malformed input.
    ///
    /// **Convention — check for the clean end BEFORE attempting to decode an
    /// element.** Exhaustion must be `Ok(None)`, never
    /// `Err(Incomplete { available: 0, .. })`: a `decode_next` that delegates
    /// straight to a [`Decode`] impl will wrongly turn an empty buffer into an
    /// error. The reference shape is:
    ///
    /// ```text
    /// if buf.is_empty() { return Ok(None); }
    /// Decode::decode(buf).map(Some)
    /// ```
    ///
    /// A *partial* element after a good start IS a real error — surfacing it
    /// (rather than silently stopping) is deliberate; the adapter fuses after
    /// the first `Err`. Consumers migrating from silent-truncation iterators
    /// should treat the newly surfaced error as the correct behavior and keep
    /// any pre-validated fast path on a separate infallible iterator.
    ///
    /// # Errors
    /// `Self::Error` if the next element is malformed.
    fn decode_next(buf: &'a [u8]) -> Result<Option<(Self, &'a [u8])>, Self::Error>;

    /// Adapter: iterate all elements, yielding `Result<Self, Self::Error>`. Stops at the
    /// first `Ok(None)` or `Err(_)`.
    #[must_use]
    fn iter(buf: &'a [u8]) -> DecodeIterator<'a, Self> {
        DecodeIterator::new(buf)
    }
}

/// Iterator produced by [`DecodeIter::iter`]. Threads the remaining buffer between calls
/// to [`DecodeIter::decode_next`]; terminates on `Ok(None)` or the first error.
pub struct DecodeIterator<'a, T> {
    buf: &'a [u8],
    done: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<'a, T> DecodeIterator<'a, T> {
    fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            done: false,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: DecodeIter<'a>> Iterator for DecodeIterator<'a, T> {
    type Item = Result<T, T::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match T::decode_next(self.buf) {
            Ok(Some((value, rest))) => {
                self.buf = rest;
                Some(Ok(value))
            }
            Ok(None) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

impl<'a, T: DecodeIter<'a>> DecodeIterator<'a, T> {
    /// Remaining element count, when [`T::WIRE_SIZE`](DecodeIter::WIRE_SIZE)
    /// is fixed. `None` for variable-width elements (or a zero `WIRE_SIZE`);
    /// `Some(0)` once the iterator has terminated.
    #[must_use]
    pub fn remaining_len(&self) -> Option<usize> {
        let w = T::WIRE_SIZE?;
        if w == 0 {
            return None;
        }
        if self.done {
            return Some(0);
        }
        Some(self.buf.len() / w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Incomplete, TrailingBytes};
    use crate::read::read_u8;

    #[derive(Debug, PartialEq)]
    enum TestErr {
        Incomplete(Incomplete),
        Trailing(TrailingBytes),
    }
    impl From<Incomplete> for TestErr {
        fn from(e: Incomplete) -> Self {
            TestErr::Incomplete(e)
        }
    }
    impl From<TrailingBytes> for TestErr {
        fn from(e: TrailingBytes) -> Self {
            TestErr::Trailing(e)
        }
    }

    // A one-byte value for the Decode contract.
    #[derive(Debug, PartialEq)]
    struct One(u8);
    impl<'a> Decode<'a> for One {
        type Error = TestErr;
        fn decode(buf: &'a [u8]) -> Result<(Self, &'a [u8]), TestErr> {
            let (b, rest) = read_u8(buf)?;
            Ok((One(b), rest))
        }
    }

    #[test]
    fn decode_exact_consumes_whole_buffer() {
        assert_eq!(One::decode_exact(&[7]).unwrap(), One(7));
    }

    #[test]
    fn decode_exact_reports_trailing_bytes() {
        let err = One::decode_exact(&[1, 2]).unwrap_err();
        assert_eq!(err, TestErr::Trailing(TrailingBytes(1)));
    }

    // A one-byte element for the DecodeIter contract; byte 0xFF simulates malformed.
    #[derive(Debug, PartialEq)]
    struct Elem(u8);
    impl<'a> DecodeIter<'a> for Elem {
        type Error = TestErr;
        fn decode_next(buf: &'a [u8]) -> Result<Option<(Self, &'a [u8])>, TestErr> {
            match buf.first() {
                None => Ok(None),
                Some(&0xFF) => Err(Incomplete {
                    needed: 2,
                    available: 1,
                }
                .into()),
                Some(&b) => Ok(Some((Elem(b), &buf[1..]))),
            }
        }
    }

    #[test]
    fn iter_yields_all_then_none() {
        let mut it = Elem::iter(&[1, 2, 3]);
        assert!(matches!(it.next(), Some(Ok(Elem(1)))));
        assert!(matches!(it.next(), Some(Ok(Elem(2)))));
        assert!(matches!(it.next(), Some(Ok(Elem(3)))));
        assert!(it.next().is_none());
    }

    #[test]
    fn iter_stops_after_first_error() {
        let mut it = Elem::iter(&[1, 0xFF, 3]);
        assert!(matches!(it.next(), Some(Ok(Elem(1)))));
        assert!(matches!(it.next(), Some(Err(TestErr::Incomplete(_)))));
        assert!(it.next().is_none());
    }

    // A 3-byte fixed-stride element advertising WIRE_SIZE (SE-2 shape: DTC record).
    #[derive(Debug, PartialEq)]
    struct Fixed3([u8; 3]);
    impl<'a> DecodeIter<'a> for Fixed3 {
        type Error = TestErr;
        const WIRE_SIZE: Option<usize> = Some(3);
        fn decode_next(buf: &'a [u8]) -> Result<Option<(Self, &'a [u8])>, TestErr> {
            if buf.is_empty() {
                return Ok(None);
            }
            let (b, rest) = crate::read::read_array::<3>(buf)?;
            Ok(Some((Fixed3(b), rest)))
        }
    }

    #[test]
    fn remaining_len_reports_count_for_fixed_stride() {
        let buf = [0u8; 12];
        let mut it = Fixed3::iter(&buf);
        assert_eq!(it.remaining_len(), Some(4));
        it.next();
        assert_eq!(it.remaining_len(), Some(3));
    }

    #[test]
    fn remaining_len_is_none_for_variable_width() {
        // Elem (above) keeps the default WIRE_SIZE = None.
        let it = Elem::iter(&[1, 2, 3]);
        assert_eq!(it.remaining_len(), None);
    }

    #[test]
    fn remaining_len_is_zero_after_exhaustion_or_error() {
        let buf = [0u8; 3];
        let mut it = Fixed3::iter(&buf);
        it.next(); // consumes the only element
        it.next(); // Ok(None) -> done
        assert_eq!(it.remaining_len(), Some(0));
    }
}
