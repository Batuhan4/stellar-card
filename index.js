#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const path = require('node:path');

const cwd = __dirname;
const targetDir = process.platform === 'win32' ? 'stellar-card.exe' : 'stellar-card';
const binaryPath = path.join(cwd, 'target', 'release', targetDir);

if (!existsSync(binaryPath)) {
  console.error('stellar-card binary not found. Run `npm install` or `npm run build` first.');
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  cwd,
  stdio: 'inherit',
  env: process.env,
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
