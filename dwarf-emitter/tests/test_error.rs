//! Integration tests for [`dwarf_emitter::error::EmitterError`].
//!
//! These are integration tests that test the public API of the error
//! type from an external consumer's perspective.

use dwarf_emitter::error::EmitterError;

// ------------------------------------------------------------------
// Creation
// ------------------------------------------------------------------

#[test]
fn test_create_unsupported_feature() {
    let err = EmitterError::UnsupportedFeature("async/await".into());
    assert_eq!(err, EmitterError::UnsupportedFeature("async/await".into()));
}

#[test]
fn test_create_io_error() {
    let err = EmitterError::Io("file not found".into());
    assert_eq!(err, EmitterError::Io("file not found".into()));
}

// ------------------------------------------------------------------
// Display impl
// ------------------------------------------------------------------

#[test]
fn test_display_unsupported_feature() {
    let err = EmitterError::UnsupportedFeature("generics".into());
    assert_eq!(err.to_string(), "unsupported feature: generics");
}

#[test]
fn test_display_unsupported_feature_empty_message() {
    let err = EmitterError::UnsupportedFeature(String::new());
    assert_eq!(err.to_string(), "unsupported feature: ");
}

#[test]
fn test_display_io_error() {
    let err = EmitterError::Io("permission denied".into());
    assert_eq!(err.to_string(), "IO error: permission denied");
}

#[test]
fn test_display_io_error_empty_message() {
    let err = EmitterError::Io(String::new());
    assert_eq!(err.to_string(), "IO error: ");
}

#[test]
fn test_display_long_messages() {
    let long_msg = "a".repeat(1000);
    let err = EmitterError::UnsupportedFeature(long_msg.clone());
    assert!(err.to_string().ends_with(&long_msg));
    assert!(err.to_string().starts_with("unsupported feature: "));
}

// ------------------------------------------------------------------
// Debug format
// ------------------------------------------------------------------

#[test]
fn test_debug_format_unsupported() {
    let err = EmitterError::UnsupportedFeature("debug".into());
    let s = format!("{err:?}");
    assert!(
        s.contains("UnsupportedFeature"),
        "Debug output should contain the variant name"
    );
    assert!(
        s.contains("debug"),
        "Debug output should contain the message"
    );
}

#[test]
fn test_debug_format_io() {
    let err = EmitterError::Io("timeout".into());
    let s = format!("{err:?}");
    assert!(
        s.contains("Io"),
        "Debug output should contain the variant name"
    );
    assert!(
        s.contains("timeout"),
        "Debug output should contain the message"
    );
}

// ------------------------------------------------------------------
// PartialEq
// ------------------------------------------------------------------

#[test]
fn test_partial_eq_same() {
    let a = EmitterError::UnsupportedFeature("x".into());
    let b = EmitterError::UnsupportedFeature("x".into());
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_message() {
    let a = EmitterError::Io("err1".into());
    let b = EmitterError::Io("err2".into());
    assert_ne!(a, b);
}

#[test]
fn test_partial_eq_different_variant() {
    let a = EmitterError::UnsupportedFeature("msg".into());
    let b = EmitterError::Io("msg".into());
    assert_ne!(a, b);
    assert_ne!(b, a);
}

// ------------------------------------------------------------------
// Clone
// ------------------------------------------------------------------

#[test]
fn test_clone_unsupported() {
    let err = EmitterError::UnsupportedFeature("feature".into());
    let cloned = err.clone();
    assert_eq!(err, cloned);
    // Verify the clone has the same value (distinct ownership)
    assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
}

#[test]
fn test_clone_io() {
    let err = EmitterError::Io("msg".into());
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

// ------------------------------------------------------------------
// Error trait
// ------------------------------------------------------------------

#[test]
fn test_error_trait_is_implemented() {
    // This compiles only if EmitterError implements std::error::Error
    fn assert_error<E: std::error::Error>() {}
    assert_error::<EmitterError>();
}

#[test]
fn test_error_trait_source() {
    let err = EmitterError::Io("source? no".into());
    // EmitterError has no inner error, so source() should be None
    let source = std::error::Error::source(&err);
    assert!(source.is_none(), "EmitterError has no inner source");
}

// ------------------------------------------------------------------
// Using errors in Result
// ------------------------------------------------------------------

#[test]
fn test_error_in_result_ok() {
    fn might_fail(ok: bool) -> Result<i32, EmitterError> {
        if ok {
            Ok(42)
        } else {
            Err(EmitterError::Io("fail".into()))
        }
    }
    assert_eq!(might_fail(true).unwrap(), 42);
}

#[test]
fn test_error_in_result_err() {
    fn might_fail() -> Result<i32, EmitterError> {
        Err(EmitterError::UnsupportedFeature("not supported".into()))
    }
    let err = might_fail().unwrap_err();
    assert_eq!(
        err,
        EmitterError::UnsupportedFeature("not supported".into())
    );
}

// ------------------------------------------------------------------
// Error interoperability with Box<dyn Error>
// ------------------------------------------------------------------

#[test]
fn test_error_can_be_boxed() {
    let err: Box<dyn std::error::Error> = Box::new(EmitterError::Io("boxed".into()));
    assert_eq!(err.to_string(), "IO error: boxed");
}

#[test]
fn test_error_downcast() {
    let err: Box<dyn std::error::Error> = Box::new(EmitterError::UnsupportedFeature("x".into()));
    let downcast = err.downcast_ref::<EmitterError>();
    assert!(downcast.is_some(), "should downcast to EmitterError");
    assert_eq!(
        *downcast.unwrap(),
        EmitterError::UnsupportedFeature("x".into())
    );
}
