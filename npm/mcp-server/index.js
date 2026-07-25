const { spawn, execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * Find the dwarf-mcp binary path.
 * Checks: local build, then npm-installed binary path.
 */
function findBinary() {
  // Check local build first (development)
  const localPaths = [
    path.join(__dirname, '..', '..', 'target', 'release', 'dwarf-mcp'),
    path.join(__dirname, '..', '..', 'target', 'debug', 'dwarf-mcp'),
  ];
  
  for (const p of localPaths) {
    if (fs.existsSync(p)) return p;
  }
  
  // Check if installed as a binary
  const installedPath = path.join(__dirname, '..', 'bin', 'dwarf-mcp');
  if (fs.existsSync(installedPath)) return installedPath;
  
  return null;
}

/**
 * Start the dwarf-mcp MCP server.
 * @param {string[]} args - CLI arguments
 * @returns {ChildProcess}
 */
function startServer(args = []) {
  const binary = findBinary();
  if (!binary) {
    throw new Error(
      'dwarf-mcp binary not found. Build it first with: cargo build -p dwarf-mcp --release'
    );
  }
  return spawn(binary, args, { stdio: 'inherit' });
}

module.exports = { findBinary, startServer };
