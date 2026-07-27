# Command Center

A local-first desktop app for saving, organizing, and instantly finding terminal commands, scripts, and technical snippets.

It exists to answer one question: *"I know I've used this command before... where the hell did I put it?"*

Your command library lives in a single SQLite file on your machine. There is no
account or sync. Optional OpenAI support is off by default and only makes a
request after you enable it and start an AI action.

---

## What it does right now

- **Command library.** Save commands, scripts, sequences, snippets, and reference notes. Title, content, description, tags, and a pile of optional fields hidden behind a disclosure so adding one entry takes about ten seconds.
- **Full-text search.** SQLite FTS5 across titles, command text, descriptions, notes, tags, and collection names. Prefix matching means results appear while you type, and punctuation-heavy searches like `rm -rf` do not blow up the query.
- **Collections and tags.** Collections say *why* commands belong together. Tags say *what* they are about. Create only the organization you actually need; fresh libraries do not come with starter collections or tags.
- **Favorites and recents.** Pin the ones you reach for. Copying an entry bumps its counter and drops it into Recent.
- **Local risk labels.** Offline rules mark entries Safe, Caution, or Destructive and explain why. `rm -rf`, `dd of=`, `git push --force`, and piping a download into a shell all get flagged before you run them. You can override the label per entry.
- **Templates.** Write `ssh {{user}}@{{host}}`, open the full entry, fill the blanks, and copy the finished command.
- **Card and compact views.** Cards are the default and form a responsive grid with up to four columns on large displays. Compact remains available when you want a denser list. Long commands scroll inside fixed previews, and a click opens the complete entry without stretching the library.
- **Three theme palettes.** Use the standard dark theme, a near-black high-contrast theme, or light mode. The system option follows your desktop preference.
- **Keyboard first.** `Ctrl + K` or `/` to search, `N` to add, arrows to move through the list, `Enter` to open an entry, and `?` for the full list. Keyboard shortcuts also sits above Settings in the sidebar so the reference is easy to find on desktop and mobile.
- **Optional AI setup.** Enable AI in Settings, choose the `gpt-5.6-luna`
  default or another model, and store your OpenAI key in the operating system
  credential manager. The key goes to the OS credential store, never to the
  database, the logs, or the frontend.
- **AI-assisted import.** Drop, pick, or paste a cheat sheet, README, or pile of
  notes. Command Center reads it locally, replaces likely secrets with
  placeholders, and shows you exactly what would be sent, down to the line each
  redaction came from. Only then can you send it. Everything that comes back is
  re-checked by the same local rules the manual editor uses, and nothing is
  saved until you review it. Import only appears when AI is on with a key
  stored.
- **Explain a saved entry.** Open an entry and ask what it actually does. You
  get a short summary, and a detailed view with the flags, the steps, what it
  changes on your machine, a safety review, and anything the answer had to
  assume. The explanation is saved locally, so opening the entry again costs
  nothing, and it joins your search. Edit the entry and its explanation is
  marked stale instead of quietly going wrong. Local risk rules still win, and
  a suggested preview command is only shown when those rules agree it is safer.

- **Ask the assistant.** A panel beside the library, or a full screen on a
  phone. Ask for a command you do not have, or open it from an entry and ask a
  follow-up about that one. Suggested commands come back as proposals with the
  local risk verdict already attached, and Command Center's reasons kept
  separate from the model's. Copy one, or send it to the normal entry form to
  review and save. Nothing runs, and nothing saves itself. The conversation
  stays in memory, so closing the window is the whole delete story.

- **Read a terminal error.** Paste what your terminal printed. Before anything
  is sent you see how much text goes out, which model receives it, and every
  likely secret that was swapped for a placeholder. What comes back is the
  likely cause, how much the output actually supports that reading, the lines
  carrying the diagnosis, and what to check next, plus what it would have needed
  to see. Pasted output is usually a fragment, and the answer says so. It opens
  a conversation, so you can ask which line meant what.

- **Convert to another shell.** Rewrite a saved entry for bash, fish, zsh, or
  PowerShell from the entry dialog. You get both commands side by side, what
  behaves differently, and what did not carry over. It is never called a
  guaranteed equivalent, because it cannot be. The rewrite carries the local
  risk verdict, and saving it creates a new entry rather than replacing the one
  you started from.

The AI features are built and tested, but none of them has been verified against
a live OpenAI account yet. See [ROADMAP.md](DOCS/ROADMAP.md) for what is left
before release.

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

Non-secret settings live in the same file. The OpenAI API key does not; Rust
stores it through Keychain, Credential Manager, or Secret Service. Backing up
the library means copying the SQLite file. The exact path is shown in Settings
→ About.

---

## Project layout

```
src/                  React frontend
  components/         UI, one concern per file
  hooks/              data loading, settings, hotkeys
  lib/                types, IPC wrappers, pure helpers
  styles/             tokens first, then everything consumes tokens
src-tauri/
  src/ai/             OpenAI client, OS credentials, model config, redaction,
                      proposal review, outbound disclosure, per-task prompts
  src/db/             schema, migrations, queries, FTS index, explanation cache
  src/import/         document parser, classification, import execution
  src/ipc/            the commands the frontend can call
  src/risk.rs         offline risk rules
  src/normalize.rs    command normalization and hashing
  tests/              end-to-end tests against a real database
```

Full technical reference: [OVERVIEW.md](DOCS/OVERVIEW.md).

---

Made with 💖 by [Pink Pixel](https://pinkpixel.dev)
