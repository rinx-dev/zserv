# zserv

[![Crates.io](https://img.shields.io/crates/v/zserv)](https://crates.io/crates/zserv)
[![npm](https://img.shields.io/npm/v/zserv)](https://www.npmjs.com/package/zserv)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**A simple, lightweight, and modern HTTP file server written in Rust.**

`zserv` is designed to be a fast and easy way to serve static files from any directory. It's perfect for development, testing, or sharing files on a local network.

## Features

- 🚀 **Fast & Lightweight**: Built with Rust and Axum for high performance.
- 📂 **Directory Listing**: Clean HTML directory listing with file icons and sizes, and a dark mode.
- 🔒 **Safe Defaults**: Hidden files like `.env` and `.git/` are neither listed nor served unless you pass `--hidden`.
- 🔄 **Always Fresh**: Responses are sent with `Cache-Control: no-cache`, so a reload always shows your latest edits.
- 🗜️ **Compression**: Text files are compressed with gzip, Brotli, or zstd. Archives and media are sent as-is, so downloads keep their size and progress bar.
- 🔧 **Configurable**: Easy CLI options for port, binding address, CORS, and hidden files.
- 🔇 **Quiet Mode**: Suppress logs with a simple flag.
- 📦 **Cross-Platform**: Binaries for Linux (x64/arm64, statically linked), macOS (x64/arm64), and Windows (x64).

## Quick Start

```bash
# Using npx (no installation required)
npx zserv

# Using bunx (no installation required)
bunx zserv

# Using cargo
cargo install zserv
zserv
```

## Installation

### NPM / Bun (Recommended)

The easiest way to run `zserv` without installing anything permanently:

```bash
# Using npx (Node.js)
npx zserv

# Using bunx (Bun)
bunx zserv
```

Or install globally:

```bash
# NPM
npm install -g zserv

# Bun
bun install -g zserv
```

### With Cargo

```bash
cargo install zserv
```

### From Binary

Download the latest pre-built binary for your operating system from the [Releases](https://github.com/rinx-dev/zserv/releases) page.

### For Developers

If you want to contribute or run from source without installing:

```bash
git clone https://github.com/rinx-dev/zserv.git
cd zserv
cargo run

# Run the tests (Rust server, then the npm launcher)
cargo test
cd npm && npm test
```

## Usage

Run `zserv` in the directory you want to serve, or specify a path:

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

By default zserv listens on all network interfaces (`0.0.0.0`), so other devices on your network can reach it. Use `-a 127.0.0.1` to keep it on your machine.

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

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
