#!/usr/bin/env node
/**
 * Detects the current OS/arch and builds the API sidecar binary for it.
 * Called by `yarn sidecar:build` and by Tauri's beforeBuildCommand.
 */

import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import path from 'path';

const root = path.resolve(fileURLToPath(import.meta.url), '../..');

const targets = {
  'darwin-arm64': 'mac-arm',
  'darwin-x64':   'mac-x64',
  'linux-x64':    'linux',
  'win32-x64':    'win',
};

const key = `${process.platform}-${process.arch}`;
const target = targets[key];

if (!target) {
  console.error(`[build-sidecar] Unsupported platform: ${key}`);
  console.error(`  Supported: ${Object.keys(targets).join(', ')}`);
  process.exit(1);
}

console.log(`[build-sidecar] Building for ${key} → sidecar:build:${target}`);

execSync(`yarn workspace @player/api run sidecar:build:${target}`, {
  stdio: 'inherit',
  cwd: root,
});
