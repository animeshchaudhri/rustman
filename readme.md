# Rustman

> A free and open source API client built in Rust with iced. Native, no webview.

I got tired of API clients that want an account, eat hundreds of megabytes, and ship a whole browser just to send an HTTP request. So I built my own. It is small, it is fast, and nothing you do leaves your machine.

**[Download](https://github.com/animeshchaudhri/rustman/releases/latest)** · **[Website](https://animeshchaudhri.github.io/rustman/)** · **[Discord](https://discord.gg/vfa8rKYzKG)**

---

## Free, and staying that way

Rustman is free and open source and it is going to stay that way.

- No paid plans, no pro tier, no "Pro" version, no subscriptions, ever.
- No account. You just open it and use it.
- No telemetry, no analytics, nothing phoning home.
- Your collections, history, and environments live on your machine and nowhere else.

This is not a startup and I am not trying to sell you anything. It is a tool I wanted for myself, so I built it and put it out for everyone.

---

## Why Rustman

| | Rustman | Postman | Bruno |
|---|---|---|---|
| Price | Free forever | Freemium / $14mo | Free |
| Install size | ~30 MB | ~350 MB | ~100 MB |
| No account | ✓ | ✗ | ✓ |
| Zero telemetry | ✓ | ✗ | ✓ |
| Native GUI (no webview) | ✓ | ✗ | ✗ |
| Native HTTP (no CORS) | ✓ | ✗ | ✗ |
| WebSocket support | ✓ | ✓ | ✗ |
| Git for collections (commit, branch, restore) | ✓ (local + your own remote) | ✗ | ✓ |
| Command palette | ✓ | ✓ | ✓ |
| Pre/post scripts | UI present, runner WIP | ✓ | ✓ |
| Large response viewer | ✓ | ✗ | ✗ |

## Features

- **Pure native GUI.** Built with [iced](https://github.com/iced-rs/iced). No webview, no Electron, no JavaScript runtime.
- **Native Rust HTTP engine.** Requests go through `reqwest`, so no browser CORS limits and no response size caps.
- **WebSocket.** Type a `ws://` or `wss://` URL and the whole panel flips into WebSocket mode. Connect, then send and receive in real time on a live feed.
- **Code editor for the body and response.** A syntax highlighted, line numbered editor (the vendored `iced-code-editor` backed by syntect) for the request body and the response, with full text selection across the whole thing.
- **File upload.** `multipart/form-data` with a real file picker. The file's bytes go to the server with a `Content-Type` guessed from the extension.
- **All the common auth types.** Bearer, Basic, API Key (header or query), Cookie, and JWT (HS256).
- **Environment variables.** `{{variable}}` substitution from the active environment, applied to the URL, headers, query params, and the JSON or Text body. The exact scope is under [What works and what does not](#what-works-and-what-does-not).
- **Collections and history.** Organize requests and replay them from history. Local SQLite is the source of truth.
- **Git for collections.** A built in Source Control panel. Commit collections (they are stored as JSON), browse the log, restore any commit back into the app (it asks first, since it overwrites local changes), make and switch branches, see the working diff, and juggle several repos. Clone, fetch, pull, and push go through your system `git`, so they use the SSH keys and logins you already have. SQLite stays the source of truth and git sits on top. More in [docs/GIT_GUI_PLAN.md](docs/GIT_GUI_PLAN.md).
- **Import.** cURL commands (native Rust tokenizer), Postman v2 and v2.1 collections, and OpenAPI specs (JSON).
- **Export.** Turn any request into a cURL command, or export a collection as Postman v2.1 JSON.
- **Command palette.** Keyboard driven access to everything.
- **Multiple request tabs.** Work on a few requests at once.
- **Session persistence.** Your open tabs come back next launch.
- **Self update.** A built in updater (`self-replace`) grabs and swaps in new releases.
- **Pre and post request scripts.** The editors are in the UI but nothing runs them yet. The test results and console panels are wired and stay empty until the runner ships.

## What works and what does not

I would rather be straight with you than oversell. A few things are half done right now.

- **Scripts (pre-request and test) are UI only.** You can write and save script text but nothing runs it. It does not touch the outgoing request, and the test results and console panels never fill in. A JavaScript runner is on the way.
- **Environment variables use the active environment only.** Substitution pulls from the one active environment and hits the URL, headers, query params, and the JSON or Text body. It does not touch auth fields (tokens, keys, passwords) or form-data fields. There are no collection, global, or dynamic (`{{$guid}}`) variables, and it resolves in a single non-recursive pass. With no environment active, `{{var}}` goes out as written.
- **Git commits are manual, and the store is two way.** No commit on save yet, but the git store does read back into the app. Restore loads a commit's collections over your current state after it asks you. Clone, fetch, pull, and push shell out to your system `git`. SQLite is still the source of truth. More in [docs/GIT_GUI_PLAN.md](docs/GIT_GUI_PLAN.md).
- **Per-request timeout is fixed at 30s.** Not configurable per request yet.

## Download

Grab the latest binary from the [releases page](https://github.com/animeshchaudhri/rustman/releases/latest).

| Platform | File |
|---|---|
| macOS (Apple Silicon + Intel) | `Rustman-macos-universal.dmg` |
| Windows x86_64 | `rustman-windows-x86_64.zip` |
| Linux x86_64 | `rustman-linux-x86_64.tar.gz` |

## Build from source

You will need a few things first.

- A recent [Rust toolchain](https://www.rust-lang.org/tools/install), 1.85 or newer. Rustman is edition 2021, but it vendors the `iced-code-editor` widget, which is edition 2024, so older toolchains will not build it.
- A C toolchain and CMake. `git2` builds with `vendored-libgit2` and `vendored-openssl`, so libgit2 and OpenSSL compile from source, and `rusqlite` bundles SQLite. You need a C compiler and `cmake` on the machine.
- A few system libraries on Linux, listed below.

```sh
git clone https://github.com/animeshchaudhri/rustman
cd rustman
cargo build --release
```

The binary ends up at `target/release/rustman`. The longer guide lives in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

### Linux system dependencies

```sh
sudo apt-get install -y \
  libxkbcommon-dev libxi-dev libx11-dev \
  libxcb1-dev libxcb-xkb-dev libdbus-1-dev \
  pkg-config cmake build-essential
```

macOS and Windows want the platform C toolchain (Xcode command line tools or MSVC build tools) plus CMake for the vendored native bits, on top of Rust.

### Development

```sh
cargo run
```

## Tech stack

- **GUI.** [iced](https://github.com/iced-rs/iced) 0.14, pure Rust, drawn with wgpu.
- **HTTP.** reqwest with rustls and multipart.
- **WebSocket.** tokio-tungstenite.
- **Storage.** rusqlite (bundled SQLite), the source of truth.
- **Collection version control.** git2 (vendored libgit2 and OpenSSL) for local commits, plus your system `git` for remotes.
- **Code editor and highlighting.** The vendored `iced-code-editor` widget backed by syntect.
- **Self update.** self-replace.
- **Scripting.** Nothing yet. A runner is planned.

## Documentation

- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) walks through building from source, the toolchain, the module layout, and how persistence works.
- [docs/GIT_GUI_PLAN.md](docs/GIT_GUI_PLAN.md) covers where the git store is today and where it is going.

## Thanks

Rustman vendors one dependency, which means a copy of its source lives right in this repo (under [`vendor/`](vendor/)) and builds from there instead of from crates.io. That way I can pin it and tweak it without waiting on a release.

- **[iced-code-editor](https://github.com/LuDog71FR/iced-code-editor)** by **LuDog71** is the syntax highlighted, line numbered editor behind the request body and the response viewer. It lives in [`vendor/iced-code-editor`](vendor/iced-code-editor) and it is MIT licensed. Thank you for building it. ❤️

And the projects this whole thing stands on, [iced](https://github.com/iced-rs/iced), [reqwest](https://github.com/seanmonstar/reqwest), [tokio](https://github.com/tokio-rs/tokio), [syntect](https://github.com/trishume/syntect), [rusqlite](https://github.com/rusqlite/rusqlite), and [git2-rs](https://github.com/rust-lang/git2-rs).

## Come say hi

Got a question, a bug, or an idea? **[Join the Discord](https://discord.gg/vfa8rKYzKG)**.

## License

MIT. See [LICENSE](LICENSE).
