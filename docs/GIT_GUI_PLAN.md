# Git GUI Plan

This doc covers where the git store is today and what is left to build. It is grounded in the current `src/services/vcs.rs` implementation and what the Source Control panel already does.

## What is built

The entire VCS layer is `src/services/vcs.rs` (roughly 160 lines over git2 via the `git2` crate).

### Source Control panel

The panel at `src/ui/sidebar/git.rs` has these features working today:

- **Manual commit.** A "Commit all" button that loops over every collection, serializes each to a JSON file, stages it, and commits. Each collection gets its own commit message `"Update collection: {name}"`.
- **Log.** Per-collection commit history, showing the last 30 commits. Each row shows the commit hash, author, date, and message.
- **Restore.** Clicking a commit in the log loads that commit's collections back into SQLite and the live app state, after a confirmation prompt.
- **Branches.** List branches, create new ones, switch between them. After switching, the working tree is read back into the app.
- **Working diff.** Shows what changed between the working tree and HEAD, per collection.
- **Multiple repos.** You can have several git repos open and switch between them. Each repo keeps its own collections, branches, and history.
- **Remote operations.** Clone a remote repo, fetch, pull, and push through your system git. These use your existing SSH keys and logins.
- **Two way read back.** Clone, open folder, pull, and branch switch read the git working tree back into the app through `load_collections_from`. SQLite stays the canonical store but the app reflects what is on disk.

### How it works

Collections are stored as one JSON file per collection in `data_dir/collections/`. Each file is `{collection_id}.json` and holds the full collection with all its requests. SQLite is always the source of truth, and git sits on top as a version layer.

Commits use a real author identity from the repo's git config (`user.name` / `user.email`). If the repo has none set, commit is blocked with an error message telling the user to configure it. The Settings panel also has fields to set git name and email directly.

Clone, fetch, pull, and push shell out to the system `git` binary (not libgit2), so they reuse whatever SSH keys and credential helpers you already have set up.

## What is not built yet

### Commit on save or delete

Saving or deleting a request only touches SQLite. There is no auto-commit. The user has to open the Source Control panel and click "Commit all" to snapshot changes. The original plan called for an optional commit-on-save setting, but it is not wired yet.

### Per-request files

Today each collection is a single JSON blob. That means a reordered request rewrites the whole file, and two people editing different requests in the same collection collide on the same file. The longer term plan is to switch to one file per request, like Bruno does, so diffs are smaller and merges are sane. See the file layout section below.

### Secret redaction

Auth tokens, passwords, API keys, and JWT secrets are serialized in plaintext into the committed JSON. There is no `.gitignore`, no redaction at write time, and no secrets file. If you push to a remote, those secrets go with it. This needs fixing before the git story is complete.

### .gitignore

No `.gitignore` is written when a repo is initialized. The app cache and any future secrets file should be excluded automatically.

### Conflict resolution

Pull and merge can produce conflicts, but the app does not detect or surface them yet. You have to resolve conflicts externally (in a terminal or git GUI) and then the app will pick up the resolved files.

### Per-repo history selection

The log view only shows history for the first collection in the repo. There is no way to browse the full repo history or select a different collection.

## Proposed on-disk format (not started)

The plan is to move from one JSON blob per collection to one file per request, in a Bruno-style folder tree. This has not been started yet.

Why per-request files:

1. Reordering rewrites huge regions. Requests live in a Vec ordered by SQLite rowid; moving one request rewrites the whole array.
2. Cross-request conflicts. Two people editing different requests in the same collection collide on the same file. Per-request files isolate the conflict to the one request both touched.
3. Churn from per-row UUIDs. Every KeyValue and FormField carries a random id; bundled, these add diff noise across the whole collection.

### Layout

```
my-api/                      # repo root (may be an existing repo)
|- .git/
|- .gitignore                # we write/append: secrets file, app caches
|- rustman/                  # collections root inside the repo (configurable)
|  |- payments/              # a collection (slug of the name)
|  |  |- collection.json     # collection metadata: id, name, order, defaults
|  |  |- environments/
|  |  |  |- staging.json     # { name, variables: { base_url: "..." } }
|  |  |  |- production.json
|  |  |- auth/               # a folder
|  |  |  |- folder.json      # { id, name, order } for stable folder ordering
|  |  |  |- login.json       # one request
|  |  |  |- refresh-token.json
|  |  |- charge.json
|  |- users/
|     |- collection.json
|     |- get-user.json
|- src/ ...                  # the rest of the user's existing repo
```

`{request}.json` is the serialized `SavedRequest` (minus `collection_id`, which is implied by location). Filenames are name slugs (`get-user.json`), not UUIDs, so the tree reads like documentation. Slug collisions get a numeric suffix (`get-user-2.json`).

### Stable keys and clean diffs

- Deterministic field order. Keep using `serde_json::to_string_pretty` (already stable by struct field order). A one-field edit yields a one-line diff.
- Ordering lives in metadata, not file order. `collection.json` and `folder.json` each carry an explicit `order: [request_id, ...]` list. Reordering requests touches only that one metadata file, never the request files.
- IDs kept but stabilised. Request/folder `id` stays in the file (it is the join key for `order` and history), but per-row `KeyValue`/`FormField` UUIDs are the churn source. Drop them from the on-disk form (re-generate on load) or make them content-derived so a re-created identical row produces no diff.
- Env files map one-to-one to `environments/{name}.json`. Secret values are redacted (see below).

### Secrets

Before any push-to-GitHub feature ships, secrets must not land in git. Two layered defences:

1. Redact at serialize time: write `{{secret_ref}}` placeholders for `bearer_token`, `basic_pass`, `api_key_value`, `jwt_secret`, `cookie_string`; resolve them from a local, git-ignored secrets file (`.rustman.secrets.json`) or from environment variables.
2. `.gitignore`: the new init/adopt flow writes/extends `.gitignore` to exclude the secrets file and any app cache.

## Source-of-truth migration (not started)

Today: SQLite canonical + git mirror. Target: git working tree canonical, with SQLite demoted to a cache/index.

### Target model

- Disk is truth. Boot calls `vcs::load_collections` and hydrates state from files. SQLite becomes a derived index for history, sessions, and fast queries.
- Edits write through to disk. Save, delete, and rename write the request file (and update the owning `order`), then mark the working tree dirty. Commit is an explicit user action.
- External changes are detected. A file watcher (or status poll on focus) notices git pull, checkout, or hand-edits and re-hydrates the affected collection.

Why hybrid, not pure files: sessions, history, and the parsed-body cache are genuinely app-local and high-churn. They should not be versioned. Keeping them in SQLite while collections live in git keeps diffs clean and avoids committing transient state.

### Data-migration path for existing users

Existing users have collections only in SQLite (`rustman.db`) and possibly a stale `data_dir/collections/{id}.json` mirror. One-time migration on first boot after the upgrade:

1. Read all collections/requests from SQLite (the proven-good source).
2. Write them out in the new per-request tree under the default location (`data_dir/collections/`, kept as an auto-managed repo for users who never pick a path).
3. `git add` + commit `"Migrate to file-based collections"` so the move is itself a recoverable point.
4. Flip canonical-source to the working tree; SQLite collection tables become a cache populated from disk on subsequent boots.
5. Stamp a `format_version` into `collection.json` so future on-disk format changes (e.g. JSON to `.bru`) can be migrated forward without guessing.

Old single-blob `{collection_id}.json` files (if present from a prior "Commit all") are detected by the loader and upgraded in place to the per-request tree.

## Git operations still to add

All new ops extend `src/services/vcs.rs` and a grown `GitMsg` (`src/message.rs`).

- **Open / adopt / clone an arbitrary repo.** Replace the hardcoded `data_dir/collections` with a folder picker (we already depend on `rfd`); detect an existing `.git`, or `git init` if absent; remember the path in settings/session. Currently only the default `data_dir/collections` path plus any repos added through the Source Control panel.
- **Per-file stage/unstage.** Today "Commit all" stages everything at once. A proper staging UI would let you pick which files go into each commit.
- **Diff view.** Show `git2::Diff` between working tree and index, index and HEAD, and commit to commit, rendered in a modal. Some diff info is shown in the panel today but a proper diff viewer is not built.
- **Conflict handling.** Required the moment pull exists. Detect via `repo.index().has_conflicts()` after a non-fast-forward merge; surface the conflicted requests, and offer ours/theirs/edit resolution per file. Per-request files would make this tractable.

## UI surface

The Source Control panel at `src/ui/sidebar/git.rs` already has more than the original plan called for. Sections from top to bottom:

- **Repository header.** Current repo path, current branch, branch switcher, and buttons for cloning and opening repos.
- **Status.** Shows modified collections with a diff summary.
- **Commit box.** Message field + "Commit all" button. Shows the author identity it will use.
- **Sync row.** Pull, Push, and Fetch buttons with error messages surfaced.
- **History.** Per-collection log with clickable commits that offer Restore.
- **Settings.** Git identity fields (name and email) in the Settings panel.

What the UI still needs:

- Per-file stage/unstage checkboxes in the status list.
- A full diff modal for viewing changes before committing.
- Conflict resolution UI for merged branches.
- Proper ahead/behind indicators against the remote.
- A "commit on save" toggle in settings.

## Phased delivery plan (remaining work)

### Phase 1 - Per-request files and secret redaction

- Switch to one file per request in a folder tree format.
- Redact secrets at serialize time and write a `.gitignore`.
- Migrate existing users from the single-blob format.
- Make the working tree canonical with SQLite as cache.

### Phase 2 - Staging, diff, and conflict resolution

- Per-file stage/unstage in the status list.
- Full diff modal with side-by-side or unified view.
- Conflict detection and ours/theirs/file-level resolution after merge.

### Phase 3 - Polish

- Commit on save/delete as an optional setting.
- Ahead/behind indicators against the remote.
- Full repo history view (not just per-collection).
- File watcher to detect external changes.
- Keyboard shortcuts for the git panel.
