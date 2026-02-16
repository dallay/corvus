#!/usr/bin/env node

const { ensureBinary } = require('../lib/install');

(async () => {
  try {
    const binaryPath = await ensureBinary();
    console.log(`[corvus] Native binary ready at ${binaryPath}`);
  } catch (error) {
    console.warn(`[corvus] Postinstall skipped: ${error.message}`);
    console.warn('[corvus] Binary will be downloaded on first run.');
  }
})();
