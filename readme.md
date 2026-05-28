# Rustman

> A free, open-source, native API testing desktop app — built entirely in Rust with iced.

No subscriptions. No cloud sync. No telemetry. No webview. Your requests stay on your machine.

**[Download](https://github.com/animeshchaudhri/rustman/releases/latest)** · **[Website](https://animeshchaudhri.github.io/rustman/)**

---

## Core Philosophy

Rustman is, and will always be, **completely free and open source**.

- No paid plans. No premium tier. No "Pro" version. No subscriptions. Ever.
- No account required to use any feature.
- No telemetry, no analytics, no phoning home.
- Collections, history, and environments stay on your machine — always.

This is not a business. It is a tool built for developers, by a developer, and given to the community for free. That will never change.

---

## Why Rustman?

| | Rustman | Postman | Bruno |
|---|---|---|---|
| Price | Free forever | Freemium / $14mo | Free |
| Install size | ~15 MB | ~350 MB | ~100 MB |
| No account | ✓ | ✗ | ✓ |
| Zero telemetry | ✓ | ✗ | ✓ |
| Native GUI (no webview) | ✓ | ✗ | ✗ |
| Native HTTP (no CORS) | ✓ | ✗ | ✗ |
| WebSocket support | ✓ | ✓ | ✗ |
| Git-based collection VCS | ✓ | ✗ | ✗ |
| Pre/post scripts | Planned | ✓ | ✓ |
| Large response viewer | ✓ | ✗ | ✗ |

## Features

- **Pure native GUI** — built with [iced](https://github.com/iced-rs/iced), zero webview, zero Electron, zero JavaScript runtime
- **Native Rust HTTP engine** — requests go through `reqwest`, no browser CORS limits or response size caps
- **WebSocket** — full-duplex connections with a live message feed, send/receive in real time
- **JSON response viewer** — syntax-highlighted, pretty-printed, with full text selection across the whole body
- **Collections & history** — organize requests, replay from history, stored in local SQLite + Git VCS
- **Git-based collection versioning** — every collection save is a real git commit; `git log` works in your data directory
- **Environment variables** — `{{variable}}` substitution in URLs, headers, and bodies
- **All auth types** — Bearer, Basic, API Key (header or query), Cookie, JWT
- **cURL import & export** — paste any cURL command; parsed by a native Rust tokenizer
- **Postman collection import** — import existing v2.1 collections and environments
- **OpenAPI import** — import any OpenAPI 3.x spec and generate requests automatically
- **Pre/post-request scripts** — _(planned: rquickjs JavaScript sandbox)_
- **Multiple request tabs** — work on several requests at the same time
- **Session persistence** — open tabs are restored on next launch

## Download

Grab the latest binary from the [releases page](https://github.com/animeshchaudhri/rustman/releases/latest).

| Platform | File |
|---|---|
| macOS (Apple Silicon + Intel) | `Rustman-macos-universal.dmg` |
| Windows x86_64 | `rustman-windows-x86_64.zip` |
| Linux x86_64 | `rustman-linux-x86_64.tar.gz` |

## Build from source

**Requirements:** [Rust toolchain](https://www.rust-lang.org/tools/install) (stable) and a few system libraries (see below).

```sh
git clone https://github.com/animeshchaudhri/rustman
cd rustman
cargo build --release
```

The binary is at `target/release/rustman`.

### Linux system dependencies

```sh
sudo apt-get install -y \
  libxkbcommon-dev libxi-dev libx11-dev \
  libxcb1-dev libxcb-xkb-dev libdbus-1-dev \
  pkg-config cmake build-essential
```

macOS and Windows have no extra requirements beyond the Rust toolchain.

### Development

```sh
cargo run
```

## Tech stack

- **GUI** — [iced](https://github.com/iced-rs/iced) 0.13 (pure Rust, wgpu-rendered)
- **HTTP** — reqwest 0.12 with rustls
- **WebSocket** — tokio-tungstenite
- **Storage** — rusqlite (SQLite, bundled)
- **Collection VCS** — git2 (libgit2)
- **Scripting** — rquickjs (QuickJS JavaScript engine, planned)
- **Syntax highlighting** — tree-sitter (JSON, JavaScript)

## License

MIT — see [LICENSE](LICENSE)
