# zserv

[![npm](https://img.shields.io/npm/v/zserv)](https://www.npmjs.com/package/zserv)
[![Crates.io](https://img.shields.io/crates/v/zserv)](https://crates.io/crates/zserv)
[![License](https://img.shields.io/badge/license-MIT-blue)](https://github.com/rinx-dev/zserv/blob/main/LICENSE)

**A simple, lightweight, and modern HTTP file server.**

`zserv` is designed to be a fast and easy way to serve static files from any directory. It's perfect for development, testing, or sharing files on a local network.

## Installation

### Quick Start (No Installation)

```bash
# Using npx (Node.js)
npx zserv

# Using bunx (Bun)
bunx zserv
```

### Global Installation

```bash
# NPM
npm install -g zserv

# Bun
bun install -g zserv
```

## Usage

```bash
zserv [OPTIONS] [PATH]
```

### Examples

```bash
# Serve current directory on port 8080
zserv

# Serve a specific directory
zserv ./public

# Specify a custom port
zserv -p 3000

# Enable CORS headers
zserv --cors

# Also list and serve dotfiles (hidden by default)
zserv --hidden

# Listen on localhost only
zserv -a 127.0.0.1
```

### Options

```text
Usage: zserv [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to serve [default: .]

Options:
  -p, --port <PORT>        Port to listen on (0 picks a free port) [default: 8080]
  -a, --address <ADDRESS>  Address to bind to [default: 0.0.0.0]
      --cors               Enable CORS headers
      --hidden             List and serve hidden files (names starting with '.')
  -s, --silent             Suppress log output
  -h, --help               Print help
  -V, --version            Print version
```

## About

This package is a small launcher with no dependencies. On first run it downloads the prebuilt binary for your platform from the matching [GitHub release](https://github.com/rinx-dev/zserv/releases), verifies its SHA-256 checksum, and caches it (next to the package, or in your user cache directory if that is read-only). The server itself is written in Rust and is also available on [crates.io](https://crates.io/crates/zserv).

Prebuilt binaries: Linux x64/arm64 (static, so Alpine works too), macOS x64/arm64, Windows x64 (also used on Windows on Arm). Requires Node.js 18 or newer.

For more information, visit the [GitHub repository](https://github.com/rinx-dev/zserv).
