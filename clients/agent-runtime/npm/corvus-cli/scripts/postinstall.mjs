#!/usr/bin/env node

import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { ensureBinary } = require('../lib/install');

try {
  const binaryPath = await ensureBinary();
  console.log(`[corvus] Native binary ready at ${binaryPath}`);
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.warn(`[corvus] Postinstall skipped: ${message}`);
  console.warn('[corvus] Binary will be downloaded on first run.');
}
