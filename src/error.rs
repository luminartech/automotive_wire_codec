//! Primitive decode error fragments. An L1 error implements `From` of each so shared
//! trait defaults and leaf helpers can construct errors generically.

/// A read ran out of bytes: `needed` were required, only `available` present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Incomplete {
    /// Number of bytes the read required.
    pub needed: usize,
    /// Number of bytes actually available.
    pub available: usize,
}

/// Bytes remained after a `decode_exact` that should have consumed the whole buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrailingBytes(pub usize);

impl core::fmt::Display for Incomplete {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "incomplete input: needed {} bytes, {} available",
            self.needed, self.available
        )
    }
}
impl core::error::Error for Incomplete {}

impl core::fmt::Display for TrailingBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} trailing bytes after decode", self.0)
    }
}
impl core::error::Error for TrailingBytes {}

/// A variable-width read/write was requested with an out-of-range byte width.
///
/// Returned (via [`ReadUintError`](crate::ReadUintError) /
/// [`WriteUintError`](crate::WriteUintError)) instead of panicking, so a
/// wire-controlled width is a recoverable *data* error, not a programming error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidWidth {
    /// Maximum width the operation supports.
    pub max: usize,
    /// Width actually requested.
    pub got: usize,
}

/// An output slice was too small for the bytes an encode needed to write.
///
/// Encode-side mirror of [`Incomplete`]. Constructed by
/// [`Encode::encode_to_slice`](crate::Encode::encode_to_slice), where both
/// counts are knowable; generic [`embedded_io::Write`] sinks cannot report
/// capacity, so they surface [`embedded_io::ErrorKind::WriteZero`] instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InsufficientBuffer {
    /// Number of bytes the encode required.
    pub needed: usize,
    /// Number of bytes the slice actually had.
    pub available: usize,
}

impl core::fmt::Display for InvalidWidth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid width: got {}, max {}", self.got, self.max)
    }
}
impl core::error::Error for InvalidWidth {}

impl core::fmt::Display for InsufficientBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "insufficient buffer: needed {} bytes, {} available",
            self.needed, self.available
        )
    }
}
impl core::error::Error for InsufficientBuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    #[test]
    fn incomplete_display() {
        let e = Incomplete {
            needed: 4,
            available: 1,
        };
        assert_eq!(
            e.to_string(),
            "incomplete input: needed 4 bytes, 1 available"
        );
    }

    #[test]
    fn trailing_display() {
        assert_eq!(
            TrailingBytes(3).to_string(),
            "3 trailing bytes after decode"
        );
    }

    fn assert_is_error<T: core::error::Error>() {}

    #[test]
    fn impl_core_error() {
        assert_is_error::<Incomplete>();
        assert_is_error::<TrailingBytes>();
    }

    #[test]
    fn invalid_width_display() {
        let e = InvalidWidth { max: 16, got: 255 };
        assert_eq!(e.to_string(), "invalid width: got 255, max 16");
    }

    #[test]
    fn insufficient_buffer_display() {
        let e = InsufficientBuffer {
            needed: 8,
            available: 4,
        };
        assert_eq!(
            e.to_string(),
            "insufficient buffer: needed 8 bytes, 4 available"
        );
    }

    #[test]
    fn new_fragments_impl_core_error() {
        assert_is_error::<InvalidWidth>();
        assert_is_error::<InsufficientBuffer>();
    }
}
