import { spawn } from "node:child_process";
import { readdir } from "node:fs/promises";
import { relative } from "node:path";
import process from "node:process";

const rootDir = new URL("..", import.meta.url);
const srcDir = new URL("../src", import.meta.url);

async function collectSpecFiles(dirUrl) {
  const entries = await readdir(dirUrl, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const entryUrl = new URL(entry.name, `${dirUrl.href}${dirUrl.href.endsWith("/") ? "" : "/"}`);
      if (entry.isDirectory()) {
        return collectSpecFiles(entryUrl);
      }
      return entry.isFile() && entry.name.endsWith(".spec.ts") ? [entryUrl] : [];
    })
  );

  return files.flat();
}

const specFileUrls = await collectSpecFiles(srcDir);
const specFiles = specFileUrls
  .map((fileUrl) => relative(rootDir.pathname, fileUrl.pathname))
  .sort((left, right) => left.localeCompare(right));

if (specFiles.length === 0) {
  console.error("No dashboard unit spec files were found under src/.");
  process.exit(1);
}

const pnpmCommand = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const vitestArgs = [
  "exec",
  "vitest",
  "--run",
  "--environment",
  "happy-dom",
  "--coverage",
  "--coverage.reporter=lcov",
  "--coverage.reporter=html",
  "--coverage.reporter=text",
  ...specFiles,
];

const child = spawn(pnpmCommand, vitestArgs, {
  cwd: rootDir,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
