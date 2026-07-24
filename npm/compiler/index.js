// @dwarf/compiler — Native Node.js addon with WASM fallback
//
// Attempts to load the native binary for the current platform.
// Falls back to the WASM compiler if no native binary is available.

function loadNative() {
  const platform = `${process.platform}-${process.arch}`;

  try {
    switch (platform) {
      case 'linux-x64':
        return require('../../dwarf-napi/dwarf-napi.node');
      case 'darwin-x64':
      case 'darwin-arm64':
      case 'win32-x64':
        // In production these would be loaded via optional deps
        // For now, fall through to WASM fallback
        throw new Error(`Platform ${platform} not yet bundled`);
      default:
        throw new Error(`Unsupported platform: ${platform}`);
    }
  } catch (e) {
    if (e.code === 'MODULE_NOT_FOUND' || e.message.includes('not yet bundled') || e.message.includes('Unsupported platform')) {
      return null;
    }
    throw e;
  }
}

function loadWasmFallback() {
  try {
    return require('@dwarf/compiler-wasm');
  } catch (e) {
    // Try local path
    try {
      return require('../compiler-wasm/dwarf_wasm.js');
    } catch (e2) {
      return null;
    }
  }
}

// Parse the JSON result from compile() into a structured object
function parseResult(jsonStr) {
  try {
    return JSON.parse(jsonStr);
  } catch {
    return {
      success: false,
      output: '',
      diagnostics: [{
        code: 'DWARF-E-JS-0001',
        severity: 'error',
        message: 'Failed to parse compiler output',
        file: null,
        line: null,
        col: null
      }],
      outputExtension: 'txt'
    };
  }
}

let native = loadNative();
let wasm = null;

const api = {
  /**
   * Compile Dwarf source code.
   * @param {string} source - Dwarf source code
   * @param {string} filename - Source filename (for error reporting)
   * @param {object|string} [options={}] - Compile options or JSON string
   * @returns {{ success, output, diagnostics, outputExtension }}
   */
  compile(source, filename, options = {}) {
    const optsJson = typeof options === 'string' ? options : JSON.stringify(options);

    if (native) {
      const result = parseResult(native.compile(source, filename, optsJson));
      if (result.success || result.diagnostics.length > 0) {
        return result;
      }
    }

    if (!wasm) {
      wasm = loadWasmFallback();
    }

    if (wasm) {
      return parseResult(wasm.compile(source, filename, optsJson));
    }

    return {
      success: false,
      output: '',
      diagnostics: [{
        code: 'DWARF-E-JS-0002',
        severity: 'error',
        message: 'No compiler backend available (native or WASM)',
        file: filename,
        line: null,
        col: null
      }],
      outputExtension: 'txt'
    };
  },

  /**
   * Simplified compile with default options (TypeScript target).
   * @param {string} source - Dwarf source code
   * @param {string} filename - Source filename
   * @returns {{ success, output, diagnostics, outputExtension }}
   */
  compileSimple(source, filename) {
    return api.compile(source, filename, { target: 'ts' });
  },

  /**
   * Get the compiler version.
   * @returns {string}
   */
  version() {
    if (native) return native.version();
    if (!wasm) wasm = loadWasmFallback();
    if (wasm) return wasm.version();
    return '0.1.0 (no backend)';
  }
};

module.exports = api;
