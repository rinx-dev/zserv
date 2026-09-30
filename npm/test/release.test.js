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
// (ustar and pax are the formats both GNU tar and bsdtar, used on macOS and Windows, accept.)
for (const format of ["ustar", "pax"]) {
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
const launcherTest = { skip: !BINARIES[`${process.platform}-${process.arch}`] };

/** Runs `fn` with a copy of the npm package whose installed binary is set up by `install`. */
function withPackage(install, fn) {
  const pkg = tempDir();
  try {
    fs.cpSync(path.join(__dirname, "..", "bin"), path.join(pkg, "bin"), { recursive: true });
    fs.cpSync(path.join(__dirname, "..", "lib"), path.join(pkg, "lib"), { recursive: true });
    fs.copyFileSync(path.join(__dirname, "..", "package.json"), path.join(pkg, "package.json"));
    install(path.join(pkg, "bin", BINARIES[`${process.platform}-${process.arch}`]));
    fn(path.join(pkg, "bin", "zserv.js"));
  } finally {
    fs.rmSync(pkg, { recursive: true, force: true, maxRetries: 3 });
  }
}

test("launcher passes arguments and exit status through", launcherTest, () => {
  // Node itself stands in for the zserv binary, so this also runs on Windows
  const installNode = (binary) => {
    try {
      fs.linkSync(process.execPath, binary);
    } catch {
      fs.copyFileSync(process.execPath, binary);
    }
  };
  withPackage(installNode, (launcher) => {
    const script = "console.log(process.argv.slice(1).join('|')); process.exitCode = 3";
    // Node stops parsing its own options at the first plain argument, so "--port" passes through
    const args = [launcher, "-e", script, "my dir", "--port", "3000"];
    const result = spawnSync(process.execPath, args, { encoding: "utf8" });
    assert.equal(result.stdout.trim(), "my dir|--port|3000");
    assert.equal(result.status, 3);
  });
});

test("launcher reports a binary that cannot start", launcherTest, () => {
  // Its interpreter does not exist on Unix, and it is not a valid .exe on Windows
  const installBroken = (binary) => fs.writeFileSync(binary, "#!/nonexistent/interpreter\n", { mode: 0o755 });
  withPackage(installBroken, (launcher) => {
    const result = spawnSync(process.execPath, [launcher, "--version"], { encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /zserv: could not run /);
  });
});

test("packing converts CRLF line endings to LF", () => {
  const pkg = tempDir();
  const files = [];
  for (const dir of ["bin", "lib"]) {
    fs.cpSync(path.join(__dirname, "..", dir), path.join(pkg, dir), { recursive: true });
    for (const name of fs.readdirSync(path.join(pkg, dir)).filter((n) => n.endsWith(".js"))) {
      files.push(path.join(pkg, dir, name));
    }
  }
  // What a Windows checkout with git's core.autocrlf produces
  for (const file of files) {
    fs.writeFileSync(file, fs.readFileSync(file, "utf8").replace(/\r?\n/g, "\r\n"));
  }

  execFileSync(process.execPath, [path.join(__dirname, "..", "scripts", "normalize-eol.js"), pkg]);

  for (const file of files) assert.ok(!fs.readFileSync(file, "utf8").includes("\r"), file);
  assert.ok(fs.readFileSync(path.join(pkg, "bin", "zserv.js"), "utf8").startsWith("#!/usr/bin/env node\n"));
});
