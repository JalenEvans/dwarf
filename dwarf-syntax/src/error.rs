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
    // Parser errors (DWARF-E-PARSE-*)
    "DWARF-E-PARSE-0001", // Expected token
    "DWARF-E-PARSE-0002", // Expected identifier
    "DWARF-E-PARSE-0003", // Unexpected token
    "DWARF-E-PARSE-0004", // Recursion depth limit exceeded
    "DWARF-E-PARSE-0005", // Invalid pattern
    // Type-checking errors (DWARF-E-TYPE-*)
    "DWARF-E-TYPE-0001", // Type mismatch (expected X, got Y)
    "DWARF-E-TYPE-0002", // Type not found
    "DWARF-E-TYPE-0003", // Argument count mismatch
    "DWARF-E-TYPE-0004", // Generic constraint violation
    "DWARF-E-TYPE-0005", // Structural field mismatch
    "DWARF-E-TYPE-0006", // Refinement constraint violation
    "DWARF-E-TYPE-0007", // Circular type definition
    "DWARF-E-TYPE-0008", // Cannot infer type
];
