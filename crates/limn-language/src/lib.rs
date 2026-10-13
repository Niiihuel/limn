//! Renderer-neutral, versioned language analysis and bounded LSP transport.
//! No process is started and no grammar is loaded until explicitly requested.
#![warn(missing_docs)]
mod coordinates;
pub mod lsp;
mod syntax;
mod worker;

pub use coordinates::{offset_to_utf16, utf16_to_offset, Position, Range};
pub use syntax::{
    AnalysisSnapshot, Cancellation, Fold, Highlight, HighlightKind, Language, ParseLimits,
    ParserEngine, Symbol, SymbolKind,
};
pub use worker::AnalysisWorker;
