#!/usr/bin/env node
"use strict";

// Launcher for the zserv binary. On first run it downloads the prebuilt binary for this
// platform from the matching GitHub release, then runs it with the same arguments.

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { BINARIES, archiveName, releaseUrl, extractBinary } = require("../lib/release.js");
const { version } = require("../package.json");

const platform = `${process.platform}-${process.arch}`;
const binary = BINARIES[platform];

main().catch((err) => fail(err.message));

async function main() {
  if (!binary) {
    fail(`no prebuilt binary for ${platform}; install from source with \`cargo install zserv\``);
  }

  // Prefer the package directory; fall back to a per-user cache when it is read-only
  // (for example a global install owned by root).
  const locations = [path.join(__dirname, binary), path.join(cacheDir(), version, binary)];
  const installed = locations.find((location) => fs.existsSync(location));
  run(installed ?? (await install(locations)));
}

async function install(locations) {
  if (typeof fetch !== "function") fail("Node.js 18 or newer is required");

  const archive = archiveName(binary);
  const url = releaseUrl(version, archive);
  process.stderr.write(`Downloading zserv ${version} for ${platform}...\n`);

  let response;
  try {
    response = await fetch(url);
  } catch (err) {
    throw new Error(`download failed: ${err.cause?.message ?? err.message}${proxyHint()}\n  ${url}`);
  }
  if (!response.ok) throw new Error(`download failed: HTTP ${response.status}\n  ${url}`);

  const data = Buffer.from(await response.arrayBuffer());
  verifyChecksum(archive, data);
  const executable = extractBinary(data, archive, binary);

  let lastError;
  for (const location of locations) {
    try {
      writeExecutable(location, executable);
      return location;
    } catch (err) {
      lastError = err;
    }
  }
  throw new Error(`could not save the zserv binary: ${lastError.message}`);
}

function verifyChecksum(archive, data) {
  // checksums.json is written when the package is published (scripts/fetch-checksums.js);
  // it is absent when running from a git checkout.
  const file = path.join(__dirname, "..", "checksums.json");
  if (!fs.existsSync(file)) return;

  const expected = JSON.parse(fs.readFileSync(file, "utf8"))[archive];
  const actual = crypto.createHash("sha256").update(data).digest("hex");
  if (actual !== expected) {
    throw new Error(
      `checksum mismatch for ${archive}: expected ${expected ?? "(none recorded)"}, got ${actual}`,
    );
  }
}

function writeExecutable(location, data) {
  fs.mkdirSync(path.dirname(location), { recursive: true });
  // Write to a temporary name and rename, so an interrupted run never leaves a truncated binary
  const tmp = `${location}.${process.pid}.tmp`;
  try {
    fs.writeFileSync(tmp, data, { mode: 0o755 });
    fs.renameSync(tmp, location);
  } catch (err) {
    fs.rmSync(tmp, { force: true });
    // Another zserv process may have just installed it (renaming over a running .exe fails)
    if (!fs.existsSync(location)) throw err;
  }
}

function run(binPath) {
  const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
  if (result.error) fail(`could not run ${binPath}: ${result.error.message}`);
  // Exit the way the binary did, so shells and scripts see the same status
  if (result.signal) process.kill(process.pid, result.signal);
  process.exit(result.status ?? 1);
}

function cacheDir() {
  const home = os.homedir();
  if (process.platform === "win32") {
    return path.join(process.env.LOCALAPPDATA || path.join(home, "AppData", "Local"), "zserv");
  }
  if (process.platform === "darwin") return path.join(home, "Library", "Caches", "zserv");
  return path.join(process.env.XDG_CACHE_HOME || path.join(home, ".cache"), "zserv");
}

function proxyHint() {
  const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
  return proxy && !process.env.NODE_USE_ENV_PROXY
    ? " (behind a proxy? set NODE_USE_ENV_PROXY=1 so Node.js uses it)"
    : "";
}

function fail(message) {
  console.error(`zserv: ${message}`);
  process.exit(1);
}
