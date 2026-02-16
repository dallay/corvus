#!/usr/bin/env node

const { spawn } = require('node:child_process');
const { ensureBinary, getAssetName } = require('../lib/install');

async function run() {
  let executable;

  try {
    executable = await ensureBinary();
  } catch (error) {
    const asset = getAssetName();
    const suffix = asset ? ` (${asset})` : '';
    console.error(`[corvus] Could not prepare native binary${suffix}: ${error.message}`);
    console.error('[corvus] You can also run Corvus with cargo: cargo run --release -- <args>');
    process.exit(1);
  }

  const child = spawn(executable, process.argv.slice(2), { stdio: 'inherit' });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 0);
  });
}

run();
