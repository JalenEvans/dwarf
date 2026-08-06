//! Type-checking error types and error codes.
//!
//! All TYPE error codes follow the format: DWARF-E-TYPE-NNNN

use dwarf_syntax::span::Span;

/// All registered TYPE error codes.
pub const TYPE_ERROR_CODES: &[&str] = &[
    "DWARF-E-TYPE-0001", // Type mismatch (expected X, got Y)
    "DWARF-E-TYPE-0002", // Type not found / unknown type
    "DWARF-E-TYPE-0003", // Argument count mismatch
    "DWARF-E-TYPE-0004", // Generic constraint violation
    "DWARF-E-TYPE-0005", // Structural field mismatch
    "DWARF-E-TYPE-0006", // Refinement constraint violation
    "DWARF-E-TYPE-0007", // Circular type definition
    "DWARF-E-TYPE-0008", // Cannot infer type (annotate explicitly)
    "DWARF-E-TYPE-0009", // Reserved
    "DWARF-E-TYPE-0010", // Reserved
    "DWARF-E-TYPE-0011", // Interface conformance (missing method / signature mismatch)
];

/// A type-checking error with structured information.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckError {
    /// The error code (e.g., "DWARF-E-TYPE-0001")
    pub code: &'static str,
    /// Human-readable error message
    pub message: String,
    /// Source location
    pub span: Span,
}

impl TypeCheckError {
    pub fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }
}
