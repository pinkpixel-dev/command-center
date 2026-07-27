# Command Center 1.0.0

Release date: July 27, 2026

Command Center 1.0.0 is the first stable release of the local-first command
library. It keeps commands, scripts, snippets, references, tags, collections,
and search data on your machine, while keeping the optional OpenAI workflows
behind an explicit opt-in and review boundary.

Building the Windows workflow does not publish a GitHub release. Release assets
still have to be reviewed and attached manually before this version is
published.

## Highlights

- Save commands, scripts, sequences, snippets, configuration fragments, and
  reference notes in a local SQLite library.
- Search content, notes, tags, collections, and current saved explanations with
  SQLite FTS5.
- Organize entries with collections, tags, favorites, recents, responsive card
  and compact views, and four theme choices.
- Keep the sidebar short while browsing complete collection and tag directories
  on their own screens.
- Select visible entries to add them to a collection without replacing their
  existing memberships, or delete the selection after confirmation.
- Fill temporary `{{named}}` placeholders, copy the rendered command, and leave
  the values out of the database.
- Review offline Safe, Caution, and Destructive labels before copying a command.
- Import inconsistent documents with OpenAI after local likely-secret redaction
  and a complete outbound disclosure.
- Explain saved entries, ask the assistant, analyze pasted terminal output, and
  convert entries between bash, fish, zsh, and PowerShell.
- Export the complete library or one collection as readable Markdown, and create
  a consistent, restorable SQLite backup.
- Open the AI assistant with `Ctrl/Cmd + Shift + K` when AI is available.
- Build Windows installers from a manually triggered GitHub Actions workflow
  without creating or publishing a release.

## AI and privacy

AI is off by default and is not required for the command library. The user
provides their own OpenAI API key, which Rust stores through the operating
system credential manager. The key is not written to SQLite, returned to the
frontend, or stored in plaintext as a fallback.

Requests use the Responses API with `store: false`. Command Center sends the
selected document, entry, question, or terminal paste needed for the action,
not the whole library. Import and terminal-error pastes show the exact outbound
size, model, and local likely-secret replacements before sending. The detector
cannot guarantee that every secret will be recognized, so the disclosure still
needs a human check.

Model suggestions are checked again by the local risk rules. Nothing is run or
saved automatically. Assistant conversations remain in memory, saved
explanations can be cleared from Settings, and conversions only become library
entries after **Review and save**.

## Fixes and improvements

- Kept the local risk verdict authoritative across imports, explanations,
  assistant proposals, terminal analysis, and shell conversion.
- Added real cancellation for assistant, diagnosis, and conversion requests.
- Added guarded recovery for an unreadable library while preserving the
  original SQLite file and sidecars.
- Added an AppImage-only GIO compatibility guard for affected CachyOS systems
  without changing normal Linux development or other package types.
- Added responsive layouts, keyboard navigation, visible focus states, and
  touch-sized controls across the main workflows.
- Expanded the in-app Help guide to cover the optional AI workflows and their
  privacy boundaries.
- Kept launch at startup and close to tray opt-in.

## Installation

After the release assets have been attached:

1. Open the
   [Command Center Releases page](https://github.com/pinkpixel-dev/command-center/releases).
2. Open version 1.0.0 and expand **Assets**.
3. Download the installer for your operating system.
4. Close any running copy of Command Center, then open the installer.

Windows users can choose the `.msi` or setup `.exe` when both are present.
Linux users can choose the `.AppImage`, `.deb`, or `.rpm` that matches their
system.

To run an AppImage:

```bash
chmod +x ./*.AppImage
./"Command Center_"*.AppImage
```

Source installation instructions are in the
[README](README.md#build-from-source).

## Upgrading

No manual database migration is required. Command Center applies its SQLite
migrations when it opens the library.

Backing up the library before an application upgrade is still sensible. Use
**Settings → Back up database** so pending WAL data is checkpointed into the
copy.

## Breaking changes

There are no known breaking data-format or configuration changes for users
upgrading from the 0.10.x line.

## Known issues and limitations

- Windows installers produced by the GitHub Actions workflow are unsigned.
  Windows SmartScreen may show an unknown-publisher warning.
- The new manual workflow builds Windows installers only. It does not build
  Linux or macOS packages and does not create or publish a GitHub release.
- AI workflows require an OpenAI API key, network access, and access to the
  selected model. Provider errors, rate limits, and model availability remain
  outside the app's control.
- Likely-secret redaction is a safety net, not a guarantee. Review the outbound
  disclosure before sending user content.
- AI responses are returned as complete structured results rather than streamed
  text.
- Command Center does not execute commands and does not provide hosted accounts
  or cloud sync.

## GitHub repository metadata

Description:

> A local-first desktop library for saving, organizing, finding, and safely
> reviewing terminal commands, scripts, and technical snippets.

Suggested topics:

`tauri`, `rust`, `react`, `typescript`, `sqlite`, `fts5`, `developer-tools`,
`terminal`, `command-line`, `local-first`, `openai`

## GitHub release

Title:

```text
Command Center 1.0.0
```

Body:

```md
Command Center 1.0.0 is the first stable release of the local-first command
library.

Save and search commands, scripts, snippets, and reference notes; organize them
with tags and collections; select entries for collection assignment or confirmed
deletion; export one collection or the whole library; fill temporary
placeholders; review local risk labels; and back up the complete database.
Optional OpenAI workflows can import messy documents, explain entries, answer
command questions, analyze terminal output, and convert between bash, fish,
zsh, and PowerShell.

AI stays off by default. User content is only sent after an explicit action,
likely secrets are replaced locally, model output goes through the local risk
rules, and nothing runs or saves itself.

Download the installer for your platform from the Assets section below.

Windows note: the current installers are unsigned, so SmartScreen may show an
unknown-publisher warning.

See the README for installation, privacy details, source builds, and the local
database location.
```
