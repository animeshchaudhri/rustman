# Rustman

> A free, open-source, lightweight API testing desktop app — built with Rust and Tauri v2.

No subscriptions. No cloud sync. No telemetry. Your requests stay on your machine.

**[Download](https://github.com/animeshchaudhri/rustman/releases/latest)** · **[Website](https://animeshchaudhri.github.io/rustman/)**

---

## Why Rustman?

| | Rustman | Postman | Bruno |
|---|---|---|---|
| Price | Free forever | Freemium / $14mo | Free |
| Install size | ~20 MB | ~350 MB | ~100 MB |
| No account | ✓ | ✗ | ✓ |
| Zero telemetry | ✓ | ✗ | ✓ |
| Native HTTP (no CORS) | ✓ | ✗ | ✗ |
| Monaco editor | ✓ | Basic | Basic |
| Async pre/post scripts | ✓ | ✓ | ✓ |
| Large response viewer | ✓ | ✗ | ✗ |

## Features

- **Native Rust HTTP engine** — requests go through `reqwest`, no browser CORS limits or size caps
- **Pre-request scripts** — async JavaScript sandbox runs before every request; set env vars, sign payloads, call `await` and WebCrypto
- **Post-request scripts** — runs after the response; chain tokens, transform data, set variables for the next request
- **Built-in test runner** — write `pm.test()` assertions in the post-request script, see pass/fail results with per-test timing
- **Collections & history** — organize requests, replay from history, all stored in local SQLite
- **Environment variables** — `{{variable}}` substitution in URLs, headers, and bodies
- **All auth types** — Bearer, Basic, API Key (header or query), Cookie, JWT
- **cURL import & export** — paste any cURL command; parsed by a native Rust tokenizer
- **Monaco editor** — VS Code's editor for request bodies and scripts, full keyboard shortcuts
- **Flexible split layout** — toggle between stacked and side-by-side panels, drag to resize
- **6 accent themes** — Flame, Violet, Cobalt, Emerald, Rose, Midnight — dark and light mode
- **Postman collection import** — import existing v2.1 collections and environments
- **Virtual response viewer** — handles multi-megabyte responses without freezing

## Download

Grab the latest installer from the [releases page](https://github.com/animeshchaudhri/rustman/releases/latest).

| Platform | Format |
|---|---|
| macOS | `.dmg` (Universal — Apple Silicon + Intel) |
| Windows | `.msi`, `.exe` |
| Linux | `.deb`, `.rpm`, `.AppImage` |

## Build from source

**Requirements:** [Rust toolchain](https://www.rust-lang.org/tools/install) + Node.js 18+ + [Tauri prerequisites](https://tauri.app/start/prerequisites/)

```sh
git clone https://github.com/animeshchaudhri/rustman
cd rustman
npm install
npm run tauri build
```

For development:

```sh
npm run tauri dev
```

## Tech stack

- **Frontend** — React, TypeScript, Vite, Tailwind CSS v4, Monaco Editor
- **Backend** — Rust, Tauri v2, reqwest, rusqlite (SQLite)
- **Scripting** — sandboxed async `Function()` constructor with Postman-compatible `pm.*` API

## License

MIT — see [LICENSE](LICENSE)
