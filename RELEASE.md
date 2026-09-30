# Release Instructions

This document describes how to release a new version of `zserv`.

## Prerequisites

1.  GitHub repository configured (`https://github.com/rinx-dev/zserv`).
2.  `crates.io` account logged in locally (`cargo login`).
3.  `npm` account logged in locally (`npm login`).
4.  CI is green on `main`.

## Release Process

### 1. Update Version

Update the version number in two files:

- `Cargo.toml`: `version = "0.X.Y"`
- `npm/package.json`: `"version": "0.X.Y"`

Then refresh `Cargo.lock` (CI and the release build use `--locked`, so a stale lockfile fails the build):

```bash
cargo check
```

Commit these changes:

```bash
git add Cargo.toml Cargo.lock npm/package.json
git commit -m "chore: bump version to 0.X.Y"
```

### 2. Create Git Tag

Create a tag for the release. The tag **must** start with `v`.

```bash
git tag v0.X.Y
git push origin v0.X.Y
```

This will trigger the GitHub Actions workflow to build binaries and create a GitHub Release.

### 3. Verify GitHub Release

Go to [GitHub Releases](https://github.com/rinx-dev/zserv/releases) and check that the new release has all five archives, each with a `.sha256` file next to it:

- `zserv-linux-amd64.tar.gz`, `zserv-linux-arm64.tar.gz`
- `zserv-macos-amd64.tar.gz`, `zserv-macos-arm64.tar.gz`
- `zserv-windows-amd64.exe.zip`

### 4. Publish to NPM

Once the GitHub Release is ready (important, because `npx zserv` downloads from there), publish the NPM package.

```bash
cd npm
npm publish --access public
```

`npm publish` first runs `scripts/fetch-checksums.js`, which records the SHA-256 of every release archive in `checksums.json` so the launcher can verify its download. It fails (and nothing is published) if any `.sha256` file is missing from the release.

### 5. Publish to Crates.io

Finally, publish the Rust crate.

```bash
cargo publish
```
