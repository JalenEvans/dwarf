//! Source location tracking.

use serde::{Deserialize, Serialize};

/// A span represents a range of bytes in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Span {
    /// Index into the source file table (file_id).
    pub file_id: usize,
    /// Byte offset of the start of the span.
    pub start: usize,
    /// Byte offset of the end of the span (exclusive).
    pub end: usize,
}

impl Span {
    /// Create a new span.
    pub fn new(file_id: usize, start: usize, end: usize) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    /// Create a zero-length span at the given position (for synthetic tokens).
    pub fn synthetic(file_id: usize, pos: usize) -> Self {
        Self::new(file_id, pos, pos)
    }
}
