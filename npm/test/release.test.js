"use strict";

const assert = require("node:assert/strict");
const { execFileSync, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { BINARIES, archiveName, extractBinary, readTarEntry } = require("../lib/release.js");

const payload = Buffer.from("zserv binary contents\n".repeat(64));

function tempDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "zserv-npm-test-"));
}

function available(command) {
  return spawnSync(command, ["--version"], { stdio: "ignore" }).status === 0;
}

test("every platform maps to a release archive", () => {
  for (const binary of Object.values(BINARIES)) {
    assert.match(archiveName(binary), /^zserv-(linux|macos|windows)-(amd64|arm64)(\.exe\.zip|\.tar\.gz)$/);
  }
});

// The pax format adds a "./PaxHeaders/<name>" metadata entry with the same base name as the
// binary, and macOS tar can add "._<name>" resource-fork entries; neither may be returned.
for (const format of ["gnu", "pax"]) {
  test(`extracts the binary from a ${format} tar.gz`, { skip: !available("tar") }, () => {
    const dir = tempDir();
    fs.writeFileSync(path.join(dir, "._zserv-linux-amd64"), "resource fork");
    fs.writeFileSync(path.join(dir, "zserv-linux-amd64"), payload);
    execFileSync(
      "tar",
      [`--format=${format}`, "-czf", "archive.tar.gz", "._zserv-linux-amd64", "zserv-linux-amd64"],
      { cwd: dir },
    );

    const archive = fs.readFileSync(path.join(dir, "archive.tar.gz"));
    assert.deepEqual(extractBinary(archive, "archive.tar.gz", "zserv-linux-amd64"), payload);
  });
}

for (const method of ["ZIP_STORED", "ZIP_DEFLATED"]) {
  test(`extracts the binary from a zip (${method})`, { skip: !available("python3") }, () => {
    const dir = tempDir();
    fs.writeFileSync(path.join(dir, "zserv-windows-amd64.exe"), payload);
    const script = [
      "import zipfile",
      `z = zipfile.ZipFile("archive.zip", "w", zipfile.${method})`,
      'z.writestr("readme.txt", "not the binary")',
      'z.write("zserv-windows-amd64.exe")',
      "z.close()",
    ].join("\n");
    execFileSync("python3", ["-c", script], { cwd: dir });

    const archive = fs.readFileSync(path.join(dir, "archive.zip"));
    assert.deepEqual(extractBinary(archive, "archive.zip", "zserv-windows-amd64.exe"), payload);
  });
}

test("reports a missing entry", () => {
  assert.throws(() => readTarEntry(Buffer.alloc(1024), "zserv-linux-amd64"), /not found/);
});

// The launcher runs an already-installed binary and must pass arguments and exit status through,
// and report (not swallow) a binary that cannot start.
const launcherTest = { skip: process.platform === "win32" || !BINARIES[`${process.platform}-${process.arch}`] };

function withFakeBinary(contents, fn) {
  const pkg = tempDir();
  fs.cpSync(path.join(__dirname, "..", "bin"), path.join(pkg, "bin"), { recursive: true });
  fs.cpSync(path.join(__dirname, "..", "lib"), path.join(pkg, "lib"), { recursive: true });
  fs.copyFileSync(path.join(__dirname, "..", "package.json"), path.join(pkg, "package.json"));
  const binary = path.join(pkg, "bin", BINARIES[`${process.platform}-${process.arch}`]);
  fs.writeFileSync(binary, contents, { mode: 0o755 });
  return fn(path.join(pkg, "bin", "zserv.js"));
}

test("launcher passes arguments and exit status through", launcherTest, () => {
  withFakeBinary('#!/bin/sh\necho "args: $*"\nexit 3\n', (launcher) => {
    const result = spawnSync(process.execPath, [launcher, "-p", "3000", "my dir"], { encoding: "utf8" });
    assert.equal(result.stdout, "args: -p 3000 my dir\n");
    assert.equal(result.status, 3);
  });
});

test("launcher reports a binary that cannot start", launcherTest, () => {
  withFakeBinary("#!/nonexistent/interpreter\n", (launcher) => {
    const result = spawnSync(process.execPath, [launcher, "--version"], { encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /zserv: could not run .*ENOENT/);
  });
});
