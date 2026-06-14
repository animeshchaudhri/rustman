# Git GUI Plan — File-Based / Git-Controlled Collections


This is the design plan for turning Rustman's collections into a first-class,
file-based, git-controlled store with a Git GUI usable by non-developers. It is
grounded in the current `src/services/vcs.rs` implementation and the gaps the VCS
audit surfaced.

---

## Goal & motivation

The most-requested capability is **git-controlled collections** — the same model
Bruno popularised. The user demand, paraphrased:

> "I want my collections to live as plain files in a git repo so I can version,
> review, and share them like code. I want a **Git GUI that a non-developer
> (support staff) can use** — see what changed, write a message, commit, and
> push/pull — without touching a terminal. I want **multiple collections per
> repo**, and I want a collection to be able to **live inside an existing repo**
> (e.g. alongside the service it tests), not only in some hidden app data dir."

Concretely that means four things today's implementation does not give us:

1. **Collections are real files on disk** that a human (and `git diff`) can read,
   in a location the user chooses — including a folder inside an existing repo.
2. **Git is the source of truth**, not a throwaway mirror. `git checkout`, an
   external edit, or a `git pull` must actually change what the app shows.
3. **A real Git GUI**: status, stage, commit (with the user's real identity),
   history with restore, branch switch, and a remote **sync** button (push/pull
   to GitHub) — all driven from the sidebar, no CLI required.
4. **Diff-friendly format** so reviews and merges are sane for teams.

---

## Where we are today

The entire VCS layer is `src/services/vcs.rs` (~160 LOC over libgit2 via the
`git2` crate). It is a **local-only, write-only mirror** of the SQLite store:

- **Repo location is hardcoded.** `open_repo` opens (or `git init`s) a repo at
  `data_dir/collections` — `src/services/vcs.rs:10-17`. `data_dir` itself is
  fixed to `dirs::data_dir()/rustman` at `src/app/boot.rs:16-18`. There is no
  path picker and no way to point at an existing repo.
- **One JSON file per collection.** `save_collection` serializes a
  `CollectionFile { collection, requests }` (the whole `Vec<SavedRequest>`) to
  `{collection_id}.json`, `index.add_path`s it, writes the tree, and commits —
  `src/services/vcs.rs:27-72`. The file is named by opaque UUID, not by name.
- **Auto-commit on demand only.** The sole writer path is the manual
  **"Commit all"** button: `src/ui/sidebar/git.rs:34-42` →
  `Message::Git(GitMsg::CommitAll)` → `src/app/update/git.rs:9-37`, which loops
  over every collection and calls `save_collection`. Each collection is its own
  commit `"Update collection: {name}"`. **There is no commit-on-save and no
  commit-on-delete** — `SaveRequest`/delete handlers only touch SQLite.
- **Per-collection log, first-collection only.** `collection_log` revwalks HEAD
  and keeps the last 50 commits whose tree contains `{id}.json`
  (`src/services/vcs.rs:136-159`); the UI shows 30 (`src/ui/sidebar/git.rs:105`).
  Both the panel-open load and the post-commit refresh hardcode
  `collections.first()` (`src/app/update/git.rs:22-26`), so multi-collection
  history is unreachable.
- **Hardcoded identity.** Every commit is authored `Rustman <rustman@local>` —
  `src/services/vcs.rs:19-21` — even though `AppState` already carries
  `github_username`/`github_email` (`src/app/boot.rs:121-122`).
- **Dead code = the missing half.** `vcs::load_collections`
  (`src/services/vcs.rs:74-90`) and `vcs::delete_collection`
  (`src/services/vcs.rs:92-127`) exist but have **zero callers**.
  `load_collections` is the only function that reads the git JSON back; because
  nothing calls it, boot loads exclusively from SQLite (`src/app/boot.rs:22-44`).
- **No remotes, branches, status, or diff.** `GitMsg` has only
  `LogLoaded / CommitAll / Committed / Error` (`src/message.rs:185-191`). The
  `git_branch` icon (`src/ui/sidebar/git.rs:36`) is decorative; the "Status"
  text is a static, **inaccurate** string `"Collections auto-committed on save"`
  (`src/ui/sidebar/git.rs:74`, and About at `:149`) — no auto-commit exists.

Net consequence (the core hazard): SQLite is canonical, git is a one-way export.
Editing files on disk, restoring an old commit, or `git pull`ing has **no effect
on the app** — the next "Commit all" silently overwrites it. Secrets
(`bearer_token`, `basic_pass`, `api_key_value`, `jwt_secret`, `cookie_string`)
are serialized in plaintext into committed JSON via `SavedRequest`
(`src/domain/collection.rs:26-36`), with no `.gitignore`.

---

## Proposed on-disk format

**Recommendation: one human-readable file per request, in a Bruno-style folder
tree, with a stable key order.** Replace the single `{collection_id}.json` blob.

### Why per-request, not single-blob

The audit's three concrete merge problems all stem from bundling every request
into one file:

1. **Reordering rewrites huge regions.** Requests live in a `Vec` ordered by
   SQLite `rowid`; moving one request rewrites the whole array.
2. **Cross-request conflicts.** Two people editing different requests in the same
   collection collide on the same file. Per-request files localise the conflict
   to the one request both touched.
3. **Churn from per-row UUIDs.** Every `KeyValue`/`FormField` carries a random
   `id`; bundled, these add diff noise across the whole collection.

Per-request files (Bruno's `.bru` model) localise every diff and conflict to the
smallest unit a human reasons about: a single request.

### Layout

A collection is a **directory**, not a file. Folders in the collection map to
subdirectories. A collection lives wherever the user points us — including a
subfolder of an existing repo.

```
my-api/                      # repo root (may be an existing repo)
├─ .git/
├─ .gitignore                # we write/append: secrets file, app caches
├─ rustman/                  # collections root inside the repo (configurable)
│  ├─ payments/              # a collection (slug of the name)
│  │  ├─ collection.json     # collection metadata: id, name, order, defaults
│  │  ├─ environments/
│  │  │  ├─ staging.json     # { name, variables: { base_url: "..." } }
│  │  │  └─ production.json
│  │  ├─ auth/               # a folder
│  │  │  ├─ folder.json      # { id, name, order } for stable folder ordering
│  │  │  ├─ login.json       # one request
│  │  │  └─ refresh-token.json
│  │  └─ charge.json
│  └─ users/
│     ├─ collection.json
│     └─ get-user.json
└─ src/ ...                  # the rest of the user's existing repo
```

`{request}.json` is the serialized `SavedRequest` (minus `collection_id`, which
is implied by location). Filenames are **name slugs** (`get-user.json`), not
UUIDs, so the tree reads like documentation. Slug collisions get a numeric
suffix (`get-user-2.json`).

### Stable keys & clean diffs

- **Deterministic field order.** Keep using `serde_json::to_string_pretty`
  (already stable by struct field order in `src/services/vcs.rs:39`). A one-field
  edit yields a one-line diff. (A `.bru`-like custom text format is *more*
  readable but costs a parser/serializer; JSON-per-request gets ~90% of the
  benefit for ~10% of the effort. Recommend JSON now, leave `.bru` as a future
  option behind a trait so the on-disk codec is swappable.)
- **Ordering lives in metadata, not file order.** `collection.json` and
  `folder.json` each carry an explicit `order: [request_id, …]` list. Reordering
  requests touches only that one metadata file, never the request files.
- **IDs kept but stabilised.** Request/folder `id` stays in the file (it's the
  join key for `order` and history), but per-**row** `KeyValue`/`FormField` UUIDs
  are the churn source — drop them from the on-disk form (re-generate on load) or
  make them content-derived so a re-created identical row produces no diff. This
  is a serializer concern, invisible to the in-memory types.
- **Env files** map one-to-one to `environments/{name}.json`. Secret values are
  redacted (see below).

### Secrets

Before any push-to-GitHub feature ships, secrets must not land in git. Two
layered defences:

1. **Redact at serialize time**: write `{{secret_ref}}` placeholders for
   `bearer_token`, `basic_pass`, `api_key_value`, `jwt_secret`, `cookie_string`;
   resolve them from a local, git-ignored secrets file (`.rustman.secrets.json`)
   or from environment variables.
2. **`.gitignore`**: the new init/adopt flow writes/extends `.gitignore` to
   exclude the secrets file and any app cache. Today `open_repo`
   (`src/services/vcs.rs:10-17`) writes no `.gitignore` at all.

---

## Source-of-truth migration

Today: **SQLite canonical + git mirror** (`src/app/boot.rs:22-44` reads SQLite;
`src/services/vcs.rs:74` `load_collections` is dead). Target: **git working tree
canonical**, with SQLite demoted to a cache/index.

### Target model (defensible hybrid)

- **Disk is truth.** Boot calls `vcs::load_collections` (resurrect the dead
  function, retarget it at the per-request tree) and hydrates `state.collections`
  / `state.requests` from files. SQLite becomes a derived index for history,
  sessions, and fast queries — rebuildable from disk, never authoritative for
  collection content.
- **Edits write through to disk.** `SaveRequest` / delete / rename write the
  request file (and update the owning `order`), then mark the working tree dirty.
  Commit is an explicit user action (see Git operations) — not silent.
- **External changes are detected.** A file watcher (or status poll on focus)
  notices `git pull` / `git checkout` / hand-edits and re-hydrates the affected
  collection, so the app reflects disk. This is the inversion the audit calls
  for: today restoring a commit has no effect.

Why hybrid, not pure-files: sessions, history, and the parsed-body cache are
genuinely app-local and high-churn — they should **not** be versioned. Keeping
them in SQLite while collections live in git keeps diffs clean and avoids
committing transient state.

### Data-migration path for existing users

Existing users have collections only in SQLite (`rustman.db`) and possibly a
stale `data_dir/collections/{id}.json` mirror. One-time migration on first boot
after the upgrade:

1. Read all collections/requests from SQLite (the proven-good source).
2. Write them out in the new per-request tree under the default location
   (`data_dir/collections/`, kept as an auto-managed repo for users who never
   pick a path).
3. `git add` + commit `"Migrate to file-based collections"` so the move is itself
   a recoverable point.
4. Flip canonical-source to the working tree; SQLite collection tables become a
   cache populated from disk on subsequent boots.
5. Stamp a `format_version` into `collection.json` so future on-disk format
   changes (e.g. JSON → `.bru`) can be migrated forward without guessing.

Old single-blob `{collection_id}.json` files (if present from a prior "Commit
all") are detected by the loader and upgraded in place to the per-request tree.

---

## Git operations to add

All new ops extend `src/services/vcs.rs` and a grown `GitMsg`
(`src/message.rs:185-191`). `git2` is already built with vendored libgit2 +
vendored OpenSSL, so HTTPS transport is available with no new dependency.

- **Open / adopt / clone an arbitrary repo.** Replace the hardcoded
  `data_dir/collections` (`src/services/vcs.rs:10-11`) with:
  - *Open existing*: a folder picker (we already depend on `rfd`); detect an
    existing `.git`, or `git init` if absent; remember the path (persist in
    settings/session — note today's session even drops `active_env_id`, so a new
    persisted field is needed).
  - *Clone*: `Repository::clone` with `FetchOptions` + auth callbacks, into a
    chosen directory, then adopt it as the collections store.
- **Status.** `repo.statuses()` → modified / staged / untracked / deleted, and
  ahead/behind vs upstream. Replaces the static, false
  `"Collections auto-committed on save"` string (`src/ui/sidebar/git.rs:74`).
- **Stage / unstage / commit with real identity.** Per-file stage
  (`index.add_path` / `index.remove_path`) and a real commit step with a
  **user-supplied message**. Author identity comes from, in order: the repo's own
  `git config` (`repo.config()` → `user.name`/`user.email`), else the app's
  `github_username`/`github_email` (`src/app/boot.rs:121-122`), else a prompt —
  never the hardcoded `Rustman <rustman@local>` (`src/services/vcs.rs:19-21`).
- **Branches / checkout.** List branches, show current, create, and checkout
  (`repo.branches`, `set_head`, `checkout_head`). After checkout, re-hydrate from
  the working tree. Add a `current_branch` field to `AppState`.
- **Diff view.** `git2::Diff` between working tree↔index, index↔HEAD, and
  commit↔commit, rendered in a modal. None exists today.
- **Remotes: push / pull / fetch.** `repo.find_remote("origin")` (+ add/set-url),
  `Remote::fetch`, a merge/fast-forward for pull, `Remote::push` for push.
  Surface ahead/behind from status. **Auth via `RemoteCallbacks::credentials`:**
  - **HTTPS + PAT**: `Cred::userpass_plaintext(username, token)` — store the PAT
    in the OS keychain, never in a committed file.
  - **SSH**: `Cred::ssh_key_from_agent` (ssh-agent) or an explicit key path with
    passphrase.
  - GitHub OAuth device-flow is a later nicety; PAT/SSH cover the
    support-staff-with-a-token case first.
- **Conflict handling.** Required the moment pull exists. Detect via
  `repo.index().has_conflicts()` after a non-fast-forward merge; surface the
  conflicted requests, and (Phase 4) offer ours/theirs/edit resolution per file.
  Per-request files make this tractable — a conflict is scoped to one request.

Also fix the two existing correctness bugs as part of this work: the swallowed
`let _ =` commit errors in `src/app/update/git.rs:18-21` (surface per-collection
failures), and the unreachable `GitMsg::Committed` (`src/app/update/git.rs:38`).

---

## UI surface

`src/ui/sidebar/git.rs` grows from "one button + a read-only log" into a real
Git panel. Proposed sections, top to bottom:

- **Repository header.** Current repo path + current branch (a real branch
  switcher dropdown, replacing the decorative `git_branch` icon at
  `src/ui/sidebar/git.rs:36`). Buttons: **Open repo…**, **Clone…**.
- **Status list.** Live `repo.statuses()` output — modified / staged / untracked,
  each row clickable to **open its diff** in a modal. Per-file **stage/unstage**
  checkboxes. Replaces the static status string (`src/ui/sidebar/git.rs:74`).
- **Commit box.** A multiline message field + **Commit** button (commits staged
  files with the resolved real identity). Shows the author it will use, so
  support staff see "committing as Jane <jane@…>".
- **Sync row.** Ahead/behind counts + **Pull** / **Push** / **Fetch** buttons,
  with a spinner and clear error toasts (PAT/SSH auth failures surfaced, not
  swallowed). A single **Sync** button can pull-then-push for the simple case.
- **History.** The existing log (`src/ui/sidebar/git.rs:105-140`) becomes
  **interactive**: per-collection (or whole-repo) history, each commit row
  offering **View diff** and **Restore this version** (checkout file/commit →
  re-hydrate). Today rows are inert. Fix the first-collection-only limitation
  (`src/app/update/git.rs:22`) so history is selectable per collection.
- **Diff modal.** Side-by-side or unified `git2::Diff` render, reused by status
  rows and history rows.
- **About/help.** Correct the misleading "Each save auto-commits" text
  (`src/ui/sidebar/git.rs:149`) to describe the actual (explicit-commit) model.

---

## Phased delivery plan

### Phase 1 — File-based foundation (open external repo, real identity, per-request files)

- **Scope.** Repo path picker + "open existing / init" + persisted path. New
  per-request on-disk format and serializer/loader (resurrect & retarget
  `load_collections`). Make the working tree canonical with a one-time
  SQLite→files migration. Real commit author from `git config` / app identity.
  `.gitignore` + secret redaction. Explicit commit (drop the false "auto-commit
  on save" language). File watcher / status-poll re-hydration.
- **Touched modules.** `src/services/vcs.rs` (format, load, identity,
  `.gitignore`, secrets), `src/app/boot.rs` (load from disk; migration),
  `src/app/update/git.rs` (commit wiring), `src/app/update/{request,sidebar}.rs`
  (write-through on save/delete), `src/message.rs` (new `GitMsg`/path messages),
  `src/ui/sidebar/git.rs` (path/identity surface), `src/domain/collection.rs`
  (serializer hooks / `format_version`), session/settings persistence for the
  repo path.
- **Risks.** Data migration is the highest-risk step — must be reversible and
  must not lose the SQLite-only collections (keep SQLite read-only during
  migration; commit the migrated tree as a recovery point). Inverting the
  source-of-truth touches boot and every save path. Secret redaction must be
  correct *before* any commit ever leaves the machine.

### Phase 2 — Local Git GUI (status / diff / stage / commit)

- **Scope.** `repo.statuses()` status list, per-file stage/unstage, commit box
  with message, `git2::Diff` modal, interactive history with **restore**
  (checkout + re-hydrate). Surface previously swallowed commit errors; emit the
  now-reachable `Committed`.
- **Touched modules.** `src/services/vcs.rs` (status, diff, checkout/restore),
  `src/message.rs` (`Status`, `Stage`, `Diff`, `Restore`, `Commit{Message}`
  variants), `src/app/update/git.rs` (handlers), `src/ui/sidebar/git.rs` +
  a new diff-modal widget.
- **Risks.** Restore/checkout mutating the working tree while the app holds
  in-memory state — must re-hydrate atomically to avoid a stale UI. Diff
  rendering perf on large collections (mitigated by per-request files).

### Phase 3 — Remotes (push / pull / fetch / clone)

- **Scope.** `Remote` add/fetch/push, pull (fetch + ff/merge), clone-to-adopt,
  ahead/behind in status, **Sync** button. Auth via
  `RemoteCallbacks::credentials` — PAT (keychain-stored) + SSH (agent/key).
  Network ops run off the UI thread as `Task::perform`.
- **Touched modules.** `src/services/vcs.rs` (remote ops + callbacks), a new
  credential/keychain helper, `src/message.rs` (`Push/Pull/Fetch/Clone`),
  `src/app/update/git.rs`, `src/ui/sidebar/git.rs` (sync row, clone dialog,
  auth prompt).
- **Risks.** Credential UX and storage (never commit a PAT; OS keychain
  integration is platform-specific). Long-running network ops need cancellation
  and clear error surfacing. Non-fast-forward pulls expose conflicts — bounded
  here to detection + a "you have conflicts, resolve in Phase 4 / externally"
  message until Phase 4 lands.

### Phase 4 — Branches & merge/conflict resolution

- **Scope.** Branch list/create/switch/checkout, current-branch display, merge,
  and per-request conflict resolution (ours/theirs/edit). Full two-way team
  workflow.
- **Touched modules.** `src/services/vcs.rs` (branches, merge, conflict
  inspection), `AppState` (`current_branch`), `src/message.rs`
  (`Branch*`, `Merge`, `Resolve*`), `src/app/update/git.rs`,
  `src/ui/sidebar/git.rs` (branch switcher) + conflict-resolution modal.
- **Risks.** Merge/conflict UX is the hardest surface to get right for
  non-developers; scope the first cut to file-level ours/theirs (per-request
  granularity keeps this manageable) and defer in-file 3-way merge. Re-hydration
  after branch switch must be reliable or the UI silently desyncs from disk.

---

*Cross-cutting cleanup folded into the phases:* delete or repurpose the dead
`vcs::delete_collection`; replace the inaccurate status/About strings; fix the
first-collection-only log; and make `panic`-prone `repo.workdir().unwrap()`
sites (`src/services/vcs.rs:24,43,44,75,98,99`) return `Result` — they currently
abort the whole process under the release `panic = "abort"` profile.
