"use strict";

// Where the prebuilt binaries live and how to unpack them. Shared by the launcher and the
// publish script; uses only Node built-ins so the package has no dependencies.

const path = require("node:path");
const zlib = require("node:zlib");

const REPO = "rinx-dev/zserv";

// `${process.platform}-${process.arch}` -> binary name inside the release archive
const BINARIES = {
  "linux-x64": "zserv-linux-amd64",
  "linux-arm64": "zserv-linux-arm64",
  "darwin-x64": "zserv-macos-amd64",
  "darwin-arm64": "zserv-macos-arm64",
  "win32-x64": "zserv-windows-amd64.exe",
  // Windows on Arm runs the x64 build through emulation
  "win32-arm64": "zserv-windows-amd64.exe",
};

function archiveName(binary) {
  return binary.endsWith(".exe") ? `${binary}.zip` : `${binary}.tar.gz`;
}

function releaseUrl(version, file) {
  return `https://github.com/${REPO}/releases/download/v${version}/${file}`;
}

/** Returns the bytes of the file called `name` inside a release archive. */
function extractBinary(archive, archiveFile, name) {
  return archiveFile.endsWith(".zip")
    ? readZipEntry(archive, name)
    : readTarEntry(zlib.gunzipSync(archive), name);
}

// The release workflow packs a single file per archive, so these readers only need to find one
// regular-file entry by name.

function readTarEntry(tar, name) {
  for (let offset = 0; offset + 512 <= tar.length; ) {
    const header = tar.subarray(offset, offset + 512);
    if (header[0] === 0) break; // end-of-archive marker

    const entryName = cString(header, 0, 100);
    const size = parseInt(cString(header, 124, 12).trim() || "0", 8);
    const type = header[156];
    const start = offset + 512;

    // '0' or NUL is a regular file; this skips PAX/GNU metadata entries, and the name check
    // skips macOS "._" resource-fork entries.
    if ((type === 0x30 || type === 0) && path.posix.basename(entryName) === name) {
      return tar.subarray(start, start + size);
    }
    offset = start + Math.ceil(size / 512) * 512;
  }
  throw new Error(`${name} not found in archive`);
}

function readZipEntry(zip, name) {
  // The end-of-central-directory record points at the list of entries
  const eocd = zip.lastIndexOf(Buffer.from("PK\x05\x06", "latin1"));
  if (eocd < 0) throw new Error("not a zip archive");
  const count = zip.readUInt16LE(eocd + 10);
  let entry = zip.readUInt32LE(eocd + 16);

  for (let i = 0; i < count; i++) {
    if (zip.readUInt32LE(entry) !== 0x02014b50) throw new Error("corrupt zip archive");
    const method = zip.readUInt16LE(entry + 10);
    const compressedSize = zip.readUInt32LE(entry + 20);
    const nameLength = zip.readUInt16LE(entry + 28);
    const extraLength = zip.readUInt16LE(entry + 30);
    const commentLength = zip.readUInt16LE(entry + 32);
    const localHeader = zip.readUInt32LE(entry + 42);
    const entryName = zip.toString("utf8", entry + 46, entry + 46 + nameLength);

    if (path.posix.basename(entryName) === name) {
      const dataStart =
        localHeader + 30 + zip.readUInt16LE(localHeader + 26) + zip.readUInt16LE(localHeader + 28);
      const data = zip.subarray(dataStart, dataStart + compressedSize);
      if (method === 0) return data;
      if (method === 8) return zlib.inflateRawSync(data);
      throw new Error(`unsupported zip compression method ${method}`);
    }
    entry += 46 + nameLength + extraLength + commentLength;
  }
  throw new Error(`${name} not found in archive`);
}

function cString(buffer, start, length) {
  const field = buffer.subarray(start, start + length);
  const end = field.indexOf(0);
  return field.toString("utf8", 0, end === -1 ? field.length : end);
}

module.exports = { BINARIES, archiveName, releaseUrl, extractBinary, readTarEntry, readZipEntry };
