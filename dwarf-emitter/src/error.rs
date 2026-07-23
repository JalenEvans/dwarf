//! Error types for the emitter framework.

use std::fmt;

/// Errors that can occur during code emission.
#[derive(Debug, Clone, PartialEq)]
pub enum EmitterError {
    /// A language feature is not supported by this backend.
    UnsupportedFeature(String),
    /// An I/O error occurred during emission.
    Io(String),
}

impl fmt::Display for EmitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitterError::UnsupportedFeature(msg) => write!(f, "unsupported feature: {}", msg),
            EmitterError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for EmitterError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // EmitterError — creation and field access
    // ------------------------------------------------------------------

    #[test]
    fn test_unsupported_feature_creation() {
        let err = EmitterError::UnsupportedFeature("async/await".into());
        assert_eq!(
            err,
            EmitterError::UnsupportedFeature("async/await".into())
        );
    }

    #[test]
    fn test_io_error_creation() {
        let err = EmitterError::Io("file not found".into());
        assert_eq!(err, EmitterError::Io("file not found".into()));
    }

    // ------------------------------------------------------------------
    // EmitterError — Display impl
    // ------------------------------------------------------------------

    #[test]
    fn test_display_unsupported_feature() {
        let err = EmitterError::UnsupportedFeature("generics".into());
        assert_eq!(
            err.to_string(),
            "unsupported feature: generics"
        );
    }

    #[test]
    fn test_display_io_error() {
        let err = EmitterError::Io("permission denied".into());
        assert_eq!(
            err.to_string(),
            "IO error: permission denied"
        );
    }

    // ------------------------------------------------------------------
    // EmitterError — Debug format
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_format() {
        let err = EmitterError::UnsupportedFeature("debug".into());
        let s = format!("{err:?}");
        assert!(s.contains("UnsupportedFeature"), "Debug should contain the variant name");
        assert!(s.contains("debug"), "Debug should contain the message");
    }

    // ------------------------------------------------------------------
    // EmitterError — PartialEq
    // ------------------------------------------------------------------

    #[test]
    fn test_partial_eq_same_variant_same_message() {
        let a = EmitterError::Io("disk full".into());
        let b = EmitterError::Io("disk full".into());
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_same_variant_different_message() {
        let a = EmitterError::Io("disk full".into());
        let b = EmitterError::Io("timeout".into());
        assert_ne!(a, b);
    }

    #[test]
    fn test_partial_eq_different_variants() {
        let a = EmitterError::UnsupportedFeature("x".into());
        let b = EmitterError::Io("x".into());
        assert_ne!(a, b);
    }

    // ------------------------------------------------------------------
    // EmitterError — Clone
    // ------------------------------------------------------------------

    #[test]
    fn test_clone_unsupported_feature() {
        let err = EmitterError::UnsupportedFeature("target".into());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_clone_io_error() {
        let err = EmitterError::Io("write failed".into());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ------------------------------------------------------------------
    // EmitterError — Error trait
    // ------------------------------------------------------------------

    #[test]
    fn test_error_trait_impl() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<EmitterError>();
    }
}
