//! DWARF-119 — Draupnir modulesing contract (call-site).
//!
//! This file pins the compile-level embedding contract for the Draupnir runtime
//! module in `dwarf-lib`. It deliberately READS a direct reference to
//! `dwarf_lib::draupnir::compile_draupnir()` so the "module not yet declared"
//! failure is expressed as a Rust compile error — the exact error a consumer
//! (`forge --draupnir`, the pipeline) would hit without the module. The Green
//! implementation resolved it by adding `pub mod draupnir;` to
//! `dwarf-lib/src/lib.rs` and a `src/draupnir.rs` module.
//!
//! Now GREEN: this file compiles because `dwarf_lib::draupnir` exists. The
//! contract holds: the module is declared and exposes the compile entry
//! point.
//!
//! Kept separate from `draupnir_tests.rs` so the compile-blocking nature of the
//! call-site does not mask that file's runtime (file-absent) failure reasons.

/// AC: `forge test --draupnir` must be able to load the Draupnir runtime. The
/// dwarf-lib library exposes a `compile_draupnir()` entry point (the analogue
/// of `dunit::compile_dunit`) that reads `runtime/draupnir/*.dwarf` and returns
/// their compiled output. The returned output must contain the `for_all` entry
/// point so property bodies can call it.
#[test]
fn test_compile_draupnir_returns_compiled_output() {
    let output = dwarf_lib::draupnir::compile_draupnir();

    assert!(
        !output.is_empty(),
        "compile_draupnir() should return non-empty compiled output (no module declared \
         today — this is the RED-phase contract)"
    );
    assert!(
        output.contains("for_all"),
        "compiled output should contain the for_all entry point, got:\n{}",
        output
    );
}
