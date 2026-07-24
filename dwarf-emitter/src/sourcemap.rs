//! Source map generation for the Dwarf emitter.
//!
//! Tracks Dwarf source positions to emitted code positions and produces
//! source map v3 JSON compatible with browser DevTools.

/// Builds a source map by recording mappings from generated positions
/// back to source positions.
pub struct SourceMapBuilder {
    builder: sourcemap::SourceMapBuilder,
    source_name: String,
}

/// The output of a compilation that may include a source map.
pub struct SourceMapOutput {
    /// The emitted code.
    pub output: String,
    /// Optional source map JSON value.
    pub source_map: Option<serde_json::Value>,
}

impl SourceMapBuilder {
    /// Create a new source map builder.
    ///
    /// `file` is the source filename (e.g., "main.kzd").
    /// `source` is the full source content (for the `sourcesContent` field).
    pub fn new(file: &str, source: &str) -> Self {
        let mut builder = sourcemap::SourceMapBuilder::new(Some(file));
        builder.add_source(file);
        builder.set_source_contents(0, Some(source));

        Self {
            builder,
            source_name: file.to_string(),
        }
    }

    /// Record a mapping from a generated position to a source position.
    ///
    /// All positions are 0-indexed (line and column).
    /// `name` is an optional identifier for the mapping (e.g., function name).
    pub fn add_mapping(
        &mut self,
        gen_line: u32,
        gen_col: u32,
        src_line: u32,
        src_col: u32,
        name: Option<&str>,
    ) {
        self.builder.add(
            gen_line,
            gen_col,
            src_line,
            src_col,
            Some(self.source_name.as_str()),
            name,
            false, // is_range
        );
    }

    /// Flush any pending data and return the source map as a JSON value.
    pub fn into_json(self) -> serde_json::Value {
        let sm = self.builder.into_sourcemap();
        let mut buf = Vec::new();
        sm.to_writer(&mut buf)
            .expect("source map serialization should not fail");
        serde_json::from_slice(&buf).expect("source map JSON should be valid")
    }

    /// Get the source filename.
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
}
