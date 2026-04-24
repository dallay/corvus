#!/usr/bin/env node

import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { ensureBinDir, ensureBinary, getAssetName } = require('../lib/install');

try {
  ensureBinDir();
  const assetName = getAssetName();
  if (!assetName) {
    throw new Error('Unsupported platform');
  }
  const binaryPath = ensureBinary();
  console.log(`[rook] Native binary ready at ${binaryPath}`);
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes('Unsupported platform')) {
    console.error(`[rook] ${message}`);
    process.exit(1);
  }
  console.error(`[rook] Postinstall failed: ${message}`);
  process.exit(1);
}
