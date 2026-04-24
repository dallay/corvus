const test = require('node:test');
const assert = require('node:assert/strict');

const {
  ensureBinary,
  getAssetNameFor,
  getPackageNameFor,
} = require('./install');

test('maps supported platforms to rook binary asset names', () => {
  assert.equal(getAssetNameFor('darwin', 'arm64'), 'rook-darwin-arm64');
  assert.equal(getAssetNameFor('darwin', 'x64'), 'rook-darwin-x64');
  assert.equal(getAssetNameFor('linux', 'x64'), 'rook-linux-x64');
  assert.equal(getAssetNameFor('linux', 'arm64'), 'rook-linux-arm64');
  assert.equal(getAssetNameFor('win32', 'x64'), 'rook-windows-x64.exe');
});

test('returns null for unsupported asset platforms', () => {
  assert.equal(getAssetNameFor('win32', 'arm64'), null);
  assert.equal(getAssetNameFor('freebsd', 'x64'), null);
});

test('maps supported platforms to rook npm package names', () => {
  assert.equal(getPackageNameFor('darwin', 'arm64'), '@dallay/rook-darwin-arm64');
  assert.equal(getPackageNameFor('darwin', 'x64'), '@dallay/rook-darwin-x64');
  assert.equal(getPackageNameFor('linux', 'x64'), '@dallay/rook-linux-x64');
  assert.equal(getPackageNameFor('linux', 'arm64'), '@dallay/rook-linux-arm64');
  assert.equal(getPackageNameFor('win32', 'x64'), '@dallay/rook-windows-x64');
});

test('ensureBinary fails clearly when native binary is not present', () => {
  assert.throws(() => ensureBinary(), /Native Rook binary is not available/);
});
