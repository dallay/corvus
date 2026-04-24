#!/usr/bin/env node

import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { ensureBinDir, getAssetName } = require('../lib/install');

try {
  const binDir = ensureBinDir();
  const assetName = getAssetName();
  if (!assetName) {
    throw new Error('Unsupported platform');
  }
  console.log(`[rook] Native binary directory ready at ${binDir}`);
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes('Unsupported platform')) {
    console.error(`[rook] ${message}`);
    process.exit(1);
  }
  console.warn(`[rook] Postinstall skipped: ${message}`);
}
