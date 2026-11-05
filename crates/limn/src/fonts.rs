//! What limn says of bytes a host gives as a font face.

use std::fmt;

/// Bytes that hold no font face limn can lay text out in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontError;

impl fmt::Display for FontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the bytes hold no font face limn can lay text out in")
    }
}

impl std::error::Error for FontError {}
