//! Exact routing identities for the one physical incoming reader.

use super::{clipboard_control::TextRequestId, OfferId};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadTag {
    Drop(OfferId),
    Text(TextRequestId),
}

impl std::fmt::Debug for ReadTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drop(id) => f.debug_tuple("Drop").field(id).finish(),
            Self::Text(_) => f.write_str("Text(<redacted>)"),
        }
    }
}
