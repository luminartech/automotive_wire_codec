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

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    #[test]
    fn incomplete_display() {
        let e = Incomplete { needed: 4, available: 1 };
        assert_eq!(e.to_string(), "incomplete input: needed 4 bytes, 1 available");
    }

    #[test]
    fn trailing_display() {
        assert_eq!(TrailingBytes(3).to_string(), "3 trailing bytes after decode");
    }

    fn assert_is_error<T: core::error::Error>() {}

    #[test]
    fn impl_core_error() {
        assert_is_error::<Incomplete>();
        assert_is_error::<TrailingBytes>();
    }
}
