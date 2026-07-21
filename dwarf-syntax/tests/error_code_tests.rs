use dwarf_syntax::error::ERROR_CODES;

#[test]
fn test_error_codes_not_empty() {
    assert!(!ERROR_CODES.is_empty(), "ERROR_CODES should contain at least one code");
}

#[test]
fn test_no_duplicate_error_codes() {
    let mut codes: Vec<&str> = ERROR_CODES.to_vec();
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), ERROR_CODES.len(), "Duplicate error codes detected!");
}

#[test]
fn test_error_code_format() {
    for code in ERROR_CODES {
        assert!(code.starts_with("DWARF-E-"), "Error code should start with DWARF-E-: {}", code);
        assert!(code.len() > 8, "Error code should have more than just prefix: {}", code);
    }
}
