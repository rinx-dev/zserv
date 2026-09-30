#!/usr/bin/env node
"use strict";

// Runs before `npm publish`: records the SHA-256 of every release archive in checksums.json,
// which ships in the package so the launcher can verify what it downloads.
// The GitHub release for this version must already exist (see RELEASE.md).

const fs = require("node:fs");
const path = require("node:path");
const { BINARIES, archiveName, releaseUrl } = require("../lib/release.js");
const { version } = require("../package.json");

async function main() {
  const archives = [...new Set(Object.values(BINARIES).map(archiveName))];
  const checksums = {};

  for (const archive of archives) {
    const url = releaseUrl(version, `${archive}.sha256`);
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status} for ${url}`);

    // sha256sum format: "<hex>  <file name>"
    const [hash] = (await response.text()).trim().split(/\s+/);
    if (!/^[0-9a-f]{64}$/i.test(hash)) throw new Error(`unexpected checksum format in ${url}`);
    checksums[archive] = hash.toLowerCase();
  }

  const file = path.join(__dirname, "..", "checksums.json");
  fs.writeFileSync(file, `${JSON.stringify(checksums, null, 2)}\n`);
  console.log(`Recorded ${archives.length} checksums for v${version} in checksums.json`);
}

main().catch((err) => {
  console.error(`fetch-checksums: ${err.message}`);
  process.exit(1);
});
