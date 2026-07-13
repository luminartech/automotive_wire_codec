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

    /// Decode the next element from the front of `buf`.
    ///
    /// Returns `Ok(Some((value, rest)))` for an element, `Ok(None)` for a clean end
    /// (buffer empty / no more elements), or `Err(_)` for malformed input.
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
}
