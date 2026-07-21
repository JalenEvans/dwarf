//! Error code definitions for the Dwarf compiler.
//! All error codes follow the format: DWARF-E-CATEGORY-NNNN

/// All registered error codes for the Dwarf compiler.
pub const ERROR_CODES: &[&str] = &[
    // Lexer errors (DWARF-E-LEX-*)
    "DWARF-E-LEX-0001", // Unexpected character
    "DWARF-E-LEX-0002", // Unterminated string literal
    "DWARF-E-LEX-0003", // Invalid integer literal
    "DWARF-E-LEX-0004", // Invalid float literal
    "DWARF-E-LEX-0005", // Non-ASCII identifier
];
