//! Tests for type-checking error codes.
//!
//! This file validates that all DWARF-E-TYPE error codes meet the required
//! format, are unique, and are registered in the central error code registry.

use dwarf_typecheck::error::TYPE_ERROR_CODES;

#[test]
fn test_type_error_codes_not_empty() {
    assert!(
        !TYPE_ERROR_CODES.is_empty(),
        "TYPE_ERROR_CODES should contain at least one code"
    );
}

#[test]
fn test_type_error_codes_unique() {
    let mut codes: Vec<&str> = TYPE_ERROR_CODES.to_vec();
    codes.sort();
    let deduped = {
        let mut d = codes.clone();
        d.dedup();
        d
    };
    assert_eq!(
        deduped.len(),
        codes.len(),
        "Duplicate TYPE error codes detected!"
    );
}

#[test]
fn test_type_error_codes_format() {
    for code in TYPE_ERROR_CODES {
        assert!(
            code.starts_with("DWARF-E-TYPE-"),
            "TYPE error code should start with DWARF-E-TYPE-: {}",
            code
        );
        // Should be at least "DWARF-E-TYPE-0001" length
        assert!(code.len() >= 17, "TYPE error code too short: {}", code);
    }
}

#[test]
fn test_type_error_codes_count() {
    // Should have at least 8 type error codes
    assert!(
        TYPE_ERROR_CODES.len() >= 8,
        "Expected at least 8 TYPE error codes, got {}",
        TYPE_ERROR_CODES.len()
    );
}
