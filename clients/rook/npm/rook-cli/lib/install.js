const fs = require('node:fs');
const path = require('node:path');

const BIN_DIR = path.join(__dirname, '..', 'bin', 'native');

function getAssetNameFor(platform, arch) {
  const matrix = {
    'darwin-arm64': 'rook-darwin-arm64',
    'darwin-x64': 'rook-darwin-x64',
    'linux-arm64': 'rook-linux-arm64',
    'linux-x64': 'rook-linux-x64',
    'win32-x64': 'rook-windows-x64.exe',
  };

  return matrix[`${platform}-${arch}`] ?? null;
}

function getPackageNameFor(platform, arch) {
  const matrix = {
    'darwin-arm64': '@dallay/rook-darwin-arm64',
    'darwin-x64': '@dallay/rook-darwin-x64',
    'linux-arm64': '@dallay/rook-linux-arm64',
    'linux-x64': '@dallay/rook-linux-x64',
    'win32-x64': '@dallay/rook-windows-x64',
  };

  return matrix[`${platform}-${arch}`] ?? null;
}

function getAssetName() {
  return getAssetNameFor(process.platform, process.arch);
}

function ensureBinary() {
  const assetName = getAssetName();
  if (!assetName) {
    throw new Error(`Unsupported platform: ${process.platform}-${process.arch}`);
  }

  const binaryPath = path.join(BIN_DIR, assetName);
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Native Rook binary is not available at ${binaryPath}`);
  }

  return binaryPath;
}

function ensureBinDir() {
  fs.mkdirSync(BIN_DIR, { recursive: true });
  return BIN_DIR;
}

module.exports = {
  BIN_DIR,
  ensureBinary,
  ensureBinDir,
  getAssetName,
  getAssetNameFor,
  getPackageNameFor,
};
