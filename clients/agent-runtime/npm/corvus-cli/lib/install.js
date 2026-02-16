const fs = require('node:fs');
const path = require('node:path');
const https = require('node:https');

const BIN_DIR = path.join(__dirname, '..', 'bin', 'native');

function getAssetName() {
  const platform = process.platform;
  const arch = process.arch;

  const matrix = {
    'darwin-arm64': 'corvus-darwin-arm64',
    'darwin-x64': 'corvus-darwin-x64',
    'linux-arm64': 'corvus-linux-arm64',
    'linux-x64': 'corvus-linux-x64',
    'win32-arm64': 'corvus-windows-arm64.exe',
    'win32-x64': 'corvus-windows-x64.exe',
  };

  return matrix[`${platform}-${arch}`] ?? null;
}

function getVersionTag() {
  const packageJsonPath = path.join(__dirname, '..', 'package.json');
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  return `v${pkg.version}`;
}

function getDownloadUrl(assetName) {
  const versionTag = getVersionTag();
  const base = process.env.CORVUS_NPM_RELEASE_BASE
    ?? 'https://github.com/dallay/corvus/releases/download';
  return `${base}/${versionTag}/${assetName}`;
}

function getTargetPath(assetName) {
  return path.join(BIN_DIR, assetName);
}

function ensureBinDir() {
  fs.mkdirSync(BIN_DIR, { recursive: true });
}

function downloadAsset(url, outPath) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (
        response.statusCode >= 300
        && response.statusCode < 400
        && response.headers.location
      ) {
        response.resume();
        downloadAsset(response.headers.location, outPath).then(resolve).catch(reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`HTTP ${response.statusCode} when fetching ${url}`));
        return;
      }

      const file = fs.createWriteStream(outPath, { mode: 0o755 });
      response.pipe(file);

      file.on('finish', () => {
        file.close(() => resolve(outPath));
      });

      file.on('error', (error) => {
        fs.rmSync(outPath, { force: true });
        reject(error);
      });
    });

    request.on('error', reject);
    request.setTimeout(20_000, () => {
      request.destroy(new Error(`Timeout downloading ${url}`));
    });
  });
}

async function ensureBinary() {
  const assetName = getAssetName();
  if (!assetName) {
    throw new Error(`Unsupported platform: ${process.platform}-${process.arch}`);
  }

  ensureBinDir();

  const targetPath = getTargetPath(assetName);
  if (fs.existsSync(targetPath)) {
    return targetPath;
  }

  const tempPath = `${targetPath}.${process.pid}.tmp`;
  const url = getDownloadUrl(assetName);

  await downloadAsset(url, tempPath);
  fs.renameSync(tempPath, targetPath);

  if (process.platform !== 'win32') {
    fs.chmodSync(targetPath, 0o755);
  }

  return targetPath;
}

module.exports = {
  ensureBinary,
  getAssetName,
};
