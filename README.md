# Command Center

A local-first desktop app for saving, organizing, and instantly finding terminal commands, scripts, and technical snippets.

It exists to answer one question: *"I know I've used this command before... where the hell did I put it?"*

Everything lives in a single SQLite file on your machine. No account, no sync, no network calls.

---

## What it does right now

- **Command library.** Save commands, scripts, sequences, snippets, and reference notes. Title, content, description, tags, and a pile of optional fields hidden behind a disclosure so adding one entry takes about ten seconds.
- **Full-text search.** SQLite FTS5 across titles, command text, descriptions, notes, tags, and collection names. Prefix matching means results appear while you type, and punctuation-heavy searches like `rm -rf` do not blow up the query.
- **Collections and tags.** Collections say *why* commands belong together (Arch Rescue, Git Mistakes). Tags say *what* they are about (docker, network, cleanup).
- **Favorites and recents.** Pin the ones you reach for. Copying an entry bumps its counter and drops it into Recent.
- **Local risk labels.** Offline rules mark entries Safe, Caution, or Destructive and explain why. `rm -rf`, `dd of=`, `git push --force`, and piping a download into a shell all get flagged before you run them. You can override the label per entry.
- **Templates.** Write `ssh {{user}}@{{host}}`, fill the blanks in the expanded card, and copy the finished command.
- **Quick Add.** A global shortcut (`Ctrl/Cmd + Shift + Space` by default) opens a small capture window from anywhere. Paste, `Ctrl + Enter`, done. It warns you when the same command is already saved.
- **Keyboard first.** `Ctrl + K` or `/` to search, `N` to add, arrows to move through the list, `Enter` to expand, `?` for the full list.

Not built yet: markdown import and the AI features. See [ROADMAP.md](ROADMAP.md).

---

## Tech

| Layer | What |
| --- | --- |
| Shell | Tauri 2 |
| Backend | Rust, `rusqlite` with bundled SQLite |
| Search | SQLite FTS5 |
| Frontend | React 19, TypeScript, Vite |
| Tests | `cargo test` + Vitest with Testing Library |

---

## Running it

You need [Rust](https://rustup.rs), Node 20+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform. On Arch that is `webkit2gtk-4.1`, `libsoup3`, and the usual build tools.

```bash
npm install
npm run tauri dev
```

To build a release bundle:

```bash
npm run tauri build
```

### Checks

```bash
npm run check       # typecheck + frontend tests + Rust tests
npm run test        # Vitest
npm run test:rust   # cargo test
npm run typecheck   # tsc --noEmit
```

---

## Where your data lives

One SQLite file in the platform app-data directory:

- Linux: `~/.local/share/dev.pinkpixel.commandcenter/library.db`
- macOS: `~/Library/Application Support/dev.pinkpixel.commandcenter/library.db`
- Windows: `%APPDATA%\dev.pinkpixel.commandcenter\library.db`

Settings live in the same file. Backing up the library means copying that one file. The exact path is shown in Settings → About.

---

## Project layout

```
src/                  React frontend
  components/         UI, one concern per file
  hooks/              data loading, settings, hotkeys
  lib/                types, IPC wrappers, pure helpers
  styles/             tokens first, then everything consumes tokens
src-tauri/
  src/db/             schema, migrations, queries, FTS index
  src/ipc/            the commands the frontend can call
  src/risk.rs         offline risk rules
  src/normalize.rs    command normalization and hashing
  tests/              end-to-end tests against a real database
```

Full technical reference: [OVERVIEW.md](OVERVIEW.md).

---

Made with 💖 by [Pink Pixel](https://pinkpixel.dev)
