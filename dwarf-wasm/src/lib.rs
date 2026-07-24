use dwarf_lib::{CompileOptions, DwarfCompiler};
use wasm_bindgen::prelude::*;

/// Compile Dwarf source code and return the result as a JSON string.
///
/// JS usage:
/// ```js
/// const result = JSON.parse(compile(source, filename, JSON.stringify({ target: "ts" })));
/// // result = { success: bool, output: string, diagnostics: [...], outputExtension: string }
/// ```
#[wasm_bindgen]
pub fn compile(source: &str, filename: &str, options_json: &str) -> String {
    // Set up panic hook for better error messages
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let options: CompileOptions = match serde_json::from_str(options_json) {
        Ok(opts) => opts,
        Err(e) => {
            return serde_json::json!({
                "success": false,
                "output": "",
                "diagnostics": [{
                    "code": "DWARF-E-WASM-0001",
                    "severity": "error",
                    "message": format!("Invalid options JSON: {}", e),
                    "file": filename,
                    "line": null,
                    "col": null
                }],
                "outputExtension": "txt"
            })
            .to_string();
        }
    };

    let compiler = DwarfCompiler::new();
    match compiler.compile(source, filename, options) {
        Ok(result) => serde_json::json!({
            "success": true,
            "output": result.output,
            "sourceMap": result.source_map,
            "diagnostics": result.diagnostics.iter().map(|d| {
                serde_json::json!({
                    "code": d.code,
                    "severity": d.severity.to_string(),
                    "message": d.message,
                    "file": d.file,
                    "line": d.line,
                    "col": d.col,
                })
            }).collect::<Vec<_>>(),
            "outputExtension": result.output_extension,
        })
        .to_string(),
        Err(errors) => {
            let diags: Vec<serde_json::Value> = errors
                .iter()
                .flat_map(|e| match e {
                    dwarf_lib::DwarfError::Compilation(diags) => diags.clone(),
                    other => vec![dwarf_lib::Diagnostic {
                        code: "DWARF-E-WASM-0002".to_string(),
                        severity: dwarf_lib::Severity::Error,
                        message: other.to_string(),
                        file: Some(filename.to_string()),
                        line: None,
                        col: None,
                    }],
                })
                .map(|d| {
                    serde_json::json!({
                        "code": d.code,
                        "severity": d.severity.to_string(),
                        "message": d.message,
                        "file": d.file,
                        "line": d.line,
                        "col": d.col,
                    })
                })
                .collect();

            serde_json::json!({
                "success": false,
                "output": "",
                "diagnostics": diags,
                "outputExtension": "txt",
            })
            .to_string()
        }
    }
}

/// Compile Dwarf source code and return the result as a JSON string.
/// Simplified version with just source and filename (uses defaults).
#[wasm_bindgen]
pub fn compile_simple(source: &str, filename: &str) -> String {
    compile(source, filename, r#"{"target":"ts"}"#)
}

/// Get the version of the compiler.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
