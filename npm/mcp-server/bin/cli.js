#!/usr/bin/env node
const { startServer } = require('../index.js');

const args = process.argv.slice(2);
try {
  const server = startServer(args);
  
  process.on('SIGINT', () => {
    server.kill('SIGINT');
    process.exit(0);
  });
  
  process.on('SIGTERM', () => {
    server.kill('SIGTERM');
    process.exit(0);
  });
  
  server.on('exit', (code) => {
    process.exit(code);
  });
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
