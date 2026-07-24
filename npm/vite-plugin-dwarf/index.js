// vite-plugin-dwarf — Compile .kzd files with HMR and source maps

const path = require('path');
const fs = require('fs');

function loadCompiler() {
  // Try native first, then WASM
  try {
    return require('@dwarf/compiler');
  } catch {
    try {
      return require('@dwarf/compiler-wasm');
    } catch {
      // Fall back to local path for development
      try {
        return require('../../npm/compiler-wasm/dwarf_wasm.js');
      } catch {
        return null;
      }
    }
  }
}

function kzdPlugin(options = {}) {
  let compiler = null;
  
  return {
    name: 'vite-plugin-dwarf',
    enforce: 'pre',
    
    // Resolve .kzd extensions
    resolveId(id, importer) {
      if (id.endsWith('.kzd')) {
        return { id: path.resolve(path.dirname(importer || process.cwd()), id) };
      }
      return null;
    },
    
    // Load .kzd files
    load(id) {
      if (id.endsWith('.kzd')) {
        try {
          return fs.readFileSync(id, 'utf-8');
        } catch (e) {
          return null;
        }
      }
      return null;
    },
    
    // Transform .kzd files
    async transform(code, id) {
      if (!id.endsWith('.kzd')) return null;
      
      if (!compiler) {
        compiler = loadCompiler();
        if (!compiler) {
          throw new Error(
            'vite-plugin-dwarf: No compiler backend found. ' +
            'Install @dwarf/compiler or @dwarf/compiler-wasm.'
          );
        }
      }
      
      const compilerOptions = {
        target: 'ts',
        source_map: true,
        pretty: options.pretty || false,
        ...options.compilerOptions
      };
      
      let result;
      try {
        // Try native API first (returns object), fall back to WASM (returns JSON string)
        if (typeof compiler.compile === 'function' && typeof compiler.compile('', '') === 'object') {
          result = compiler.compile(code, id, JSON.stringify(compilerOptions));
        } else if (typeof compiler.compile === 'function') {
          const jsonResult = compiler.compile(code, id, JSON.stringify(compilerOptions));
          result = JSON.parse(jsonResult);
        } else {
          const jsonResult = compiler.compile_simple(code, id);
          result = JSON.parse(jsonResult);
        }
      } catch (e) {
        // If options JSON failed, try without
        try {
          const jsonResult = compiler.compile_simple(code, id);
          result = JSON.parse(jsonResult);
        } catch (e2) {
          this.error(`Dwarf compilation error: ${e2.message}`);
          return null;
        }
      }
      
      if (!result.success && result.diagnostics && result.diagnostics.length > 0) {
        const diag = result.diagnostics[0];
        this.error({
          message: `[${diag.code}] ${diag.message}`,
          id,
          loc: diag.line != null ? { line: diag.line + 1, column: (diag.col || 0) + 1 } : undefined
        });
        return null;
      }
      
      // Return transformed code with source map
      let map = null;
      if (result.sourceMap) {
        try {
          map = typeof result.sourceMap === 'string' 
            ? JSON.parse(result.sourceMap) 
            : result.sourceMap;
        } catch {
          map = null;
        }
      }
      
      return {
        code: result.output || '',
        map
      };
    },
    
    // HMR handler
    handleHotUpdate(ctx) {
      if (!ctx.file.endsWith('.kzd')) return;
      
      const timestamp = Date.now();
      const modulePath = ctx.file.replace(/\\/g, '/');
      
      // Read and recompile
      const code = fs.readFileSync(ctx.file, 'utf-8');
      
      let result;
      try {
        const jsonResult = compiler.compile_simple(code, ctx.file);
        result = JSON.parse(jsonResult);
      } catch (e) {
        // On error, do a full page reload
        ctx.server.ws.send({ type: 'full-reload' });
        return;
      }
      
      if (!result.success) {
        ctx.server.ws.send({ type: 'full-reload' });
        return;
      }
      
      // Send hot update
      ctx.server.ws.send({
        type: 'update',
        updates: [{
          type: 'js-update',
          path: modulePath + `?t=${timestamp}`,
          acceptedPath: modulePath,
          timestamp
        }]
      });
      
      // Return empty array to prevent default HMR handling
      return [];
    },
    
    // Add .kzd to the list of known asset extensions
    configureServer(server) {
      // Ensure .kzd files are served as JS modules
      server.middlewares.use((req, res, next) => {
        if (req.url && req.url.endsWith('.kzd')) {
          // Let Vite handle it through the transform pipeline
          req.url = req.url.replace(/\.kzd$/, '.kzd.js');
        }
        next();
      });
    }
  };
}

module.exports = kzdPlugin;
module.exports.default = kzdPlugin;
