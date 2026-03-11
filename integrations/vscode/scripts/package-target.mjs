#!/usr/bin/env node
// Package a platform-specific .vsix with the syntaqlite binary bundled in server/.
//
// Usage:
//   node scripts/package-target.mjs --target <vscode-target> --binary <path-to-binary>
//
// Example:
//   node scripts/package-target.mjs --target darwin-arm64 --binary ../../target/release/syntaqlite
//
// VS Code platform targets:
//   darwin-arm64, darwin-x64, linux-arm64, linux-x64, win32-x64

import { execSync } from "child_process";
import { copyFileSync, mkdirSync, chmodSync, existsSync, rmSync } from "fs";
import { basename, resolve } from "path";

const args = process.argv.slice(2);
function getArg(name) {
  const idx = args.indexOf(`--${name}`);
  if (idx === -1 || idx + 1 >= args.length) {
    console.error(`Missing --${name} argument`);
    process.exit(1);
  }
  return args[idx + 1];
}

const target = getArg("target");
const binaryPath = resolve(getArg("binary"));

if (!existsSync(binaryPath)) {
  console.error(`Binary not found: ${binaryPath}`);
  process.exit(1);
}

// Prepare server/ directory with the binary
const serverDir = resolve("server");
mkdirSync(serverDir, { recursive: true });

const binaryName = target.startsWith("win32") ? "syntaqlite.exe" : "syntaqlite";
const destPath = resolve(serverDir, binaryName);

console.log(`Copying ${binaryPath} -> ${destPath}`);
copyFileSync(binaryPath, destPath);

if (!target.startsWith("win32")) {
  chmodSync(destPath, 0o755);
}

// Package with vsce
console.log(`Packaging for target: ${target}`);
try {
  execSync(`npx @vscode/vsce package --target ${target}`, {
    stdio: "inherit",
  });
} finally {
  // Clean up server/ directory
  rmSync(serverDir, { recursive: true, force: true });
}
