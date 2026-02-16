const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const https = require('node:https');

const BIN_DIR = path.join(__dirname, '..', 'bin', 'native');
const MAX_REDIRECTS = 5;

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
  const baseOverride = process.env.CORVUS_NPM_RELEASE_BASE;
  const baseValue = baseOverride
    ?? 'https://github.com/dallay/corvus/releases/download';

  let baseUrl;
  try {
    baseUrl = new URL(baseValue);
  } catch (error) {
    throw new Error(`Invalid CORVUS_NPM_RELEASE_BASE URL: ${baseValue}`);
  }

  if (!['https:', 'http:'].includes(baseUrl.protocol)) {
    throw new Error(`Unsupported download URL protocol: ${baseUrl.protocol}`);
  }

  if (baseUrl.protocol === 'http:' && baseOverride) {
    console.warn(
      `[corvus] Insecure CORVUS_NPM_RELEASE_BASE detected (${baseOverride}). `
      + 'Downloads will use HTTP and may be intercepted.',
    );
  }

  const normalizedBase = baseUrl.href.replace(/\/+$/, '');
  return `${normalizedBase}/${versionTag}/${assetName}`;
}

function getTargetPath(assetName) {
  return path.join(BIN_DIR, assetName);
}

function ensureBinDir() {
  fs.mkdirSync(BIN_DIR, { recursive: true });
}

function downloadAsset(url, outPath, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    if (redirectCount > MAX_REDIRECTS) {
      reject(new Error(`Too many redirects while fetching ${url}`));
      return;
    }

    let parsedUrl;
    try {
      parsedUrl = new URL(url);
    } catch (error) {
      reject(new Error(`Invalid download URL: ${url}`));
      return;
    }

    const client = parsedUrl.protocol === 'http:' ? http : https;
    const timeoutMs = 20_000;
    let settled = false;
    let file;

    const finalize = (error, value) => {
      if (settled) {
        return;
      }
      settled = true;
      if (file) {
        file.destroy();
        fs.rmSync(outPath, { force: true });
      }
      if (error) {
        reject(error);
        return;
      }
      resolve(value);
    };

    const request = client.get(parsedUrl, (response) => {
      response.on('error', (error) => {
        finalize(error);
      });

      if (
        response.statusCode >= 300
        && response.statusCode < 400
        && response.headers.location
      ) {
        const redirectUrl = new URL(response.headers.location, parsedUrl).toString();
        response.resume();
        request.setTimeout(0);
        settled = true;
        downloadAsset(redirectUrl, outPath, redirectCount + 1).then(resolve).catch(reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        request.setTimeout(0);
        reject(new Error(`HTTP ${response.statusCode} when fetching ${url}`));
        return;
      }

      file = fs.createWriteStream(outPath, { mode: 0o755 });
      file.on('finish', () => {
        file.close((closeError) => {
          request.setTimeout(0);
          if (closeError) {
            finalize(closeError);
            return;
          }
          const completedPath = outPath;
          file = null;
          finalize(null, completedPath);
        });
      });

      file.on('error', (error) => {
        request.setTimeout(0);
        finalize(error);
      });

      response.pipe(file);
    });

    request.on('error', (error) => {
      request.setTimeout(0);
      finalize(error);
    });

    request.setTimeout(timeoutMs, () => {
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
