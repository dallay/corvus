#!/usr/bin/env node

const { ensureBinary } = require('../lib/install');

ensureBinary()
  .then((binaryPath) => {
    console.log(`[corvus] Native binary ready at ${binaryPath}`);
  })
  .catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.warn(`[corvus] Postinstall skipped: ${message}`);
    console.warn('[corvus] Binary will be downloaded on first run.');
  });
