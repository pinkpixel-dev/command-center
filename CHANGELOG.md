# Changelog

All notable changes to Command Center are recorded here. This project follows
[semantic versioning](https://semver.org/).

## Unreleased

### 🧹 Dependencies

- Updated compatible npm and Cargo packages, including React 19.3, Tauri 2.12,
  Vite 7.3, Vitest 4.1, reqwest 0.13.5, and Tokio 1.53.
- Removed the known npm advisories in the development toolchain and patched
  the Rust TLS advisories in `rustls` and the optional `quinn-proto` path.
- Enabled Vitest's explicit mock-history cleanup so module mocks do not leak
  calls between tests under Vitest 4.

## 1.2.0 - September 7, 2026

Command Center now runs as a self-hosted server as well as a desktop app. Same
library features, same two AI providers, reachable from a browser on your
phone. The desktop app is unchanged in how it behaves.

The two installs keep separate libraries. There is no sync between them, and
the Markdown export and import is the only bridge, deliberately.

### 🖥️ Self-hosted server

- Added a server that runs in Docker, keeps its SQLite library on a mounted
  volume, and serves the same interface over HTTP.
- Every command the desktop app calls is available as `POST /api/<name>` with
  the same argument object, and `GET /api/events` carries the same live
  notifications the desktop app listens for.
- Importing takes an upload and exporting answers with the file, because the
  desktop versions name a path on the machine you are sitting at.
- `GET /api/health` reports the schema version as well as liveness, so a
  server answering while its migrations failed does not read as healthy.

### 🔐 Sign-in

- The server asks for a password, set through `COMMAND_CENTER_PASSWORD` or a
  file named by `COMMAND_CENTER_PASSWORD_FILE`. It refuses to start without
  one, and refuses a password under eight characters.
- A session lasts thirty days and renews on use, held in an `HttpOnly`,
  `SameSite=Lax` cookie. `Secure` is added when a proxy in front reports
  HTTPS, so a plain LAN deployment can still sign in.
- Three wrong passwords are free. After that the wait doubles from one second
  and stops at thirty, which slows guessing to a crawl without ever locking
  the owner out.
- Losing a session closes the library rather than leaving it on screen going
  stale.

### 🐳 Docker

- Added a multi-stage `Dockerfile`, a `docker-compose.yml`, and an entrypoint
  that checks the mounted volume is writable and says exactly what to fix when
  it is not.
- The image installs the Codex CLI, so nothing on the host is needed for it.
- One volume covers the library and the Codex login, because both sit under
  the data directory. A restart does not sign the Codex account out.
- Codex sign-in uses the device-code flow, since a server has no browser to
  open. Browser mode is refused with a message saying to use the code instead.

### 🧩 One frontend, two applications

- Split the IPC layer behind a platform interface that Vite resolves at build
  time. The desktop bundle carries no HTTP client and the web bundle carries
  no Tauri code.
- `npm run build:web` builds the web bundle; `npm run dev:web` serves it and
  proxies the API to a running server.

### 🐛 Fixes

- Cancelling a Codex sign-in could never have worked: the handler asked Tauri
  for a `CodexService` while the managed value was an `Arc<CodexService>`, so
  the lookup could not match. It now asks for the type that is managed.
- A malformed request body came back as plain text rather than the error shape
  the app parses, so it surfaced as an unhelpful status code.

### 🔧 Codex 0.149.1 and newer

- Fixed the Codex provider failing to connect on Codex 0.149.1 and newer. That
  release retired `approval_policy = "untrusted"` as a config key, and because
  Command Center starts Codex with `--strict-config`, a retired key is a
  startup failure by design. The startup flag is now `approval_policy =
  "never"`, which in Codex's own safety check is the value that refuses
  anything the sandbox will not allow rather than asking a user who is not
  there. Threads still start with the stricter `untrusted`, which is accepted
  over the wire. Verified against Codex 0.147.0, 0.148.0, and 0.153.4.
- This affected the desktop app too, not only the container. Anyone whose Codex
  had updated past 0.148.0 had a provider that reported ready and then would
  not connect.

## 1.1.0 - August 22, 2026

The ChatGPT / Codex provider: discovery, the app-server connection, the account
connection, the live model list, and all six AI actions routed through
whichever provider is selected.

### 🔌 Codex provider

- Added Codex discovery with a supported-version floor of 0.147.0. It checks a
  saved path, then `PATH`, then known install locations, and reports a specific
  reason when it cannot use what it finds. A saved path is authoritative and
  never falls back to a different Codex.
- Added the app-server process manager and JSON-RPC client: bounded
  newline-delimited reads, request-to-response matching, notification routing,
  a bounded restart counter, and clean shutdown when the app exits.
- Every server request is answered rather than ignored, and is declined.
  Ignoring one leaves Codex waiting and can wedge a turn.
- The app server starts with `--strict-config`, so a Codex update that renames
  or removes a hardening setting fails loudly at startup instead of silently
  dropping it.
- The process gets an allowlisted environment and Command Center's own
  `CODEX_HOME`, so connecting here never touches the user's own Codex login.

### 🔑 Connecting ChatGPT

- Added Connect ChatGPT. Codex owns the sign-in; Command Center opens the
  browser and waits. The authorization address never reaches the interface.
- Added a sign-in code option for when the browser cannot open or finish, and
  a Cancel action that ends the attempt rather than abandoning it.
- Added Disconnect, with a confirmation that says the user's own Codex CLI
  sign-in is separate and stays as it is.
- Added the live model list from the connected account. There is no invented
  default: if the list cannot be read it stays empty and says why, and a saved
  model that disappears has to be re-chosen.

### ⚙️ Settings

- Added an AI provider choice. Existing installations stay on the OpenAI API
  key provider, and the two providers keep separate model selections.
- Added a Codex panel showing whether Codex is installed, too old, or unusable,
  with the install command, a Check again action, and a manual path field.
- Added a provider router. All six AI actions now reach whichever provider is
  selected, through one place that resolves the provider and its model.
- AI entry points are gated by the selected provider's own readiness rather
  than by whether an API key happens to be stored.
- The Import and error-analysis disclosure screens now name the model that will
  actually receive the text, rather than the other provider's saved choice.

### 🛡️ Tool containment

- A Codex task runs as one ephemeral thread in an empty private directory with
  a read-only sandbox, and any command, file change, or tool call fails the
  turn and discards the result. Unknown item types are refused too, so a future
  Codex release cannot gain the ability to act by default.
- Usage limits, expired connections, and context overruns each get their own
  message. Codex error text is never repeated, because it can carry an address
  or an account identifier.

### 📖 Documentation

- Documented both providers in the README: what each one is, how to set it up,
  what leaves the machine, and that Codex has to be installed separately
  because the executable is around 250 MB.
- Said plainly that a connected ChatGPT account follows that plan's usage
  limits and is not free API access.

### 🧹 Interface text

- The Codex program path field now appears only when it is needed: when Codex
  was not found, is too old, cannot be used, or when a path is already saved.
  A working install shows its version and nothing to configure.
- Moved the Codex explanations out of the panel and into Help, which now
  describes both providers, what each one stores, and why signing in to
  Command Center does not change the account the Codex CLI uses.

### 🧪 Tests

- Added integration tests that run against a real Codex installation and skip
  with a reason where none is present. They cover the handshake, hardening
  overrides, an unauthenticated account read, and clean shutdown.

## 1.0.3 - August 22, 2026

Revised the ChatGPT / Codex provider plan after checking its assumptions
against a real Codex CLI 0.147.0 install. The provider is still not implemented
or enabled.

### 📝 Planning

- Replaced the bundled Codex sidecar with detection of a Codex installation
  already present on the machine, plus a documented minimum version. The
  official Linux x86_64 executable measures 247 MB uncompressed, which is too
  much weight for an optional provider in a small local-first app.
- Recorded the bundling approach as a rejected alternative in an appendix
  instead of deleting the analysis.
- Rewrote the tool-prevention release gate. Codex has no switch that removes
  its shell or file tools, so the gate is now client-side containment: a
  read-only sandbox in an empty private directory, declined approvals, and any
  tool item failing its turn.
- Corrected the approval-policy reading. `approval_policy = "never"` means
  never ask and always run, which is the most permissive option.
- Added `--strict-config` to the launch plan so a Codex update that renames a
  hardening key fails loudly at startup instead of silently dropping it.
- Corrected the claim about Codex writing nothing to disk. An isolated Codex
  home still receives memory, goals, log, queue, and state databases plus shell
  snapshots, so the plan and future Help text describe separation, not absence.
- Recorded that `thread/start` opens an upstream connection, so offline
  failures surface at thread creation rather than at turn start.
- Confirmed against the generated schema that ephemeral threads, per-turn
  output schemas, and keyring-only credential storage all work as planned.

### 🏷️ Versioning

- Bumped the documentation-planning release from 1.0.2 to 1.0.3.

## 1.0.2 - August 22, 2026

Documented the approved direction for a bundled Codex sidecar and a separate
ChatGPT-backed AI provider. This release does not implement or enable the
provider.

### 📝 Planning

- Added the complete provider plan for sidecar packaging, ChatGPT connection,
  provider-specific models, all six AI workflows, platform limits, tests, and
  release gates.
- Kept Codex OAuth and refresh tokens outside Command Center. The plan requires
  an isolated Codex home, keyring-only storage, ephemeral threads, and no tool
  execution.
- Recorded Linux and Windows as the first sidecar package targets. Mobile keeps
  the existing API-key path because `codex app-server` requires a local process.

### 🏷️ Versioning

- Bumped the documentation-planning release from 1.0.1 to 1.0.2.

## 1.0.1 - July 27, 2026

An interface cleanup pass. The app explained itself in a lot of places that did
not need explaining, so the standing paragraphs are gone and the field hints
that repeated their own control are gone with them. Help, tooltips, and every
error or status message are untouched.

### 📖 Documentation

- Documented both providers in the README: what each one is, how to set it up,
  what leaves the machine, and that Codex has to be installed separately
  because the executable is around 250 MB.
- Said plainly that a connected ChatGPT account follows that plan's usage
  limits and is not free API access.

### 🧹 Interface text

- Removed the assistant disclosure line under the composer. It restated the
  active model and the redaction and accuracy caveats on every message.
- Removed the AI settings paragraph about where requests go and where the key
  is stored, the Data section paragraph about export and backup, and the
  standing paragraph above the Saved explanations row.
- Removed the secret-detection caveat and the post-send explanation from both
  outbound disclosures, in Import and in terminal-error analysis.
- Removed the review-screen paragraph about re-checking model output, and the
  paste-box paragraph about how much output to include.
- Dropped the explanatory subtitles from the Settings, Import, All collections,
  and All tags headers. The library header keeps its live entry count.
- Dropped the field hints that restated their control: startup registration,
  tray behavior, library view, AI opt-in, model choice, key handling, risk
  detection, favorites, and collection membership.
- Shortened the placeholder and tag hints to `Use {{placeholders}} for parts
  that change` and `Enter or comma to add`, and the import badge to
  `From the model`.

### 🏷️ Models

- Replaced the `gpt-5-nano` fixtures in the settings and AI tests with current
  models, and took the model name out of the import token-budget comment, so
  nothing reads as though nano were the default. `gpt-5.6-luna` is still the
  default and is still first in the curated list.
- `gpt-5-nano` remains selectable in the model dropdown.

### 🧹 Maintenance

- Removed the now-unused `model` prop from `AssistantDock` and `AssistantPanel`
  along with its `contextNote` helper, and the CSS rules left orphaned by the
  removed paragraphs.
- Removed the two tests that only asserted the assistant disclosure text.

## 1.0.0 - July 27, 2026

The first stable release closes the post-AI roadmap with library-scale
navigation, real bulk operations, collection exports, complete user guidance,
and repeatable Windows installer builds. The optional AI workflows were also
checked against a real OpenAI account before the release documentation was
updated.

### 🗂️ Library organization

- Limited the sidebar to six Collections and eighteen Tags, while keeping the
  active item visible and providing dedicated, mobile-friendly View all pages.
- Added Select mode to Cards and Compact views. Selected entries can be added
  to a collection without losing existing memberships or deleted after one
  confirmation.
- Made both bulk operations atomic in SQLite. A stale entry id fails the whole
  operation instead of leaving a partly updated library.
- Added collection-specific Markdown export from the active collection menu,
  including readable empty-collection exports and every member beyond the
  normal interactive result limit.

### ⌨️ Help and shortcuts

- Added `Ctrl/Cmd + Shift + K` to open or close the AI assistant when AI is
  available.
- Expanded Help with AI setup, key storage, outbound disclosure, Import,
  Explain and its local cache, assistant proposals, terminal-error analysis,
  shell conversion, and the boundaries that keep commands from running or
  saving themselves.

### 📦 Release

- Promoted the project from 0.10.0 to 1.0.0 across the root package and Rust
  crate metadata.
- Added a manually triggered, read-only Windows workflow that builds MSI and
  NSIS installers as downloadable workflow artifacts without creating a
  GitHub release.
- Rewrote the README around release downloads, then source development, with a
  centered project logo, three real screenshots, and complete AI/privacy
  details.
- Added focused 1.0.0 release notes, installation guidance, known limitations,
  and copyable GitHub release text.

### 🧪 Verification

- Confirmed connection, inconsistent-document extraction, explanations,
  assistant answers, incomplete terminal-error analysis, and conversions
  across bash, fish, zsh, and PowerShell with a real OpenAI account.
- Added real SQLite integration tests for bulk assignment, bulk deletion,
  rollback on stale selections, search freshness, cleanup, and
  collection-scoped export.
- Frontend coverage now includes both selection views, accessible bulk
  controls, sidebar limits and active-item preservation, organization browsers,
  collection export, the assistant shortcut gate, and the expanded Help copy.

## 0.10.0 - July 27, 2026

Phase 7C finishes the optional AI work. Paste what your terminal printed and get
a likely cause with checks worth trying, or rewrite a saved entry for another
shell. Both keep the same boundary everything else has: you see what leaves the
machine before it does, the local risk rules have the last word, and nothing
runs or saves itself.

### 🩺 Terminal errors

- Added Read an error to the assistant panel, reachable from its mode switch or
  from the command palette. Paste the output, see exactly what would be sent,
  then send it.
- The disclosure is the same one Import shows: outbound size, the model
  receiving it, and every likely secret listed by kind, line, and placeholder.
  Redaction runs once and the text described is the text sent.
- An analysis says what failed, the likeliest cause, how much the output
  actually supports that reading, the lines carrying the diagnosis, and what to
  check next. It also says what it would have needed to see, because pasted
  output is usually a fragment.
- The analysis opens a conversation rather than ending one. Follow-up questions
  carry the paste with its line numbering intact, so asking which line said
  something has an answer.
- A suggested next step is usually a diagnostic, arrives as a proposal, and goes
  through the same local risk check, Copy, and Review and save as every other
  suggestion.

### 🔀 Shell conversion

- Added Convert to another shell inside the full entry dialog, beside Explain.
  Converts between bash, fish, zsh, and PowerShell.
- The picker leaves out the shell the entry already uses, and converting an
  entry to its own shell is refused before any request is made.
- The original and the rewrite are shown together, with what behaves
  differently and what did not carry over listed separately.
- A conversion is never presented as a guaranteed equivalent. The result says
  how close it is, and there is deliberately no value meaning "exact" for it to
  claim.
- The converted command carries the local risk verdict, with Command Center's
  reasons and the model's labelled separately. Review and save opens the normal
  entry form, so the original entry is never overwritten.

### 🔒 Same boundaries as before

- Both surfaces stay hidden unless AI is on with a key stored, and the backend
  refuses either call while AI is off.
- Every proposed command now reaches the screen through one shared review path,
  so the local rules cannot be skipped by any workflow.
- Stop aborts the request for real rather than abandoning it, and the three
  cancellable flows draw request ids from one counter so they cannot collide.
- Nothing about an analysis or a conversion is written to the database.

### 🧹 Maintenance

- Split the prompt module into one file per workflow. Instruction wording stays
  per-task; only the proposed-command schema is shared.
- Moved proposal review and the outbound disclosure into their own modules, so
  import, the assistant, error analysis, and conversion share one implementation
  of each instead of four.
- Bumped the prompt revision to `2026-07-27.1`.
- Rust tests 230 to 274, frontend tests 161 to 192.

## 0.9.0 - July 26, 2026

Phase 7B adds the command assistant. Ask for a command you do not have, or ask
a follow-up about one you saved. Suggested commands arrive as proposals with a
local risk check attached, and the only way one reaches the library is through
the normal entry form.

### 💬 Assistant

- Added the assistant panel: a docked column beside the library on desktop, a
  full-screen sheet once the window is narrow enough that the sidebar becomes a
  drawer. It is not a modal, so the library stays readable while you wait.
- Reachable from the sidebar, the command palette, and an Ask button in the
  full entry dialog. All three appear only when AI is enabled with a key stored.
- Opening it from an entry carries that entry as context. The panel names the
  entry above the transcript, and asking about a different one starts fresh.
- Send, Stop, Send it again, Clear conversation, Copy, and Review and save all
  have real loading, error, and empty states. Enter sends, Shift+Enter is a
  newline, Escape closes the panel.

### 🧾 Proposed commands

- A suggested command arrives in its own field rather than buried in prose, so
  it always reaches the local risk rules before it reaches the screen.
- Each proposal shows its effective risk level, with Command Center's reasons
  and the model's listed separately and labelled by source.
- The level is the higher of the two verdicts, so a model cannot talk a
  destructive command down.
- Review and save opens the normal entry form, prefilled and with the reviewed
  risk level. Nothing is saved from the panel, and nothing is ever run.
- Copy puts the command on the clipboard without touching any entry's copy
  count, because a proposal is not a library entry.

### 🛑 Stopping a request

- Stop actually aborts the request rather than looking away from it. The network
  call runs in its own task, so cancelling drops the HTTP request instead of
  leaving it running and billed.
- Cancelling after the answer has already arrived is not treated as a failure.

### 🔒 Privacy

- The conversation lives in memory for as long as the window is open. Nothing is
  written to SQLite, and `store: false` keeps nothing with OpenAI either.
- Turning AI off closes the panel and discards the conversation.
- The panel sends an entry id, not entry content, so the model only ever sees
  what the library actually holds.
- The question, the conversation being resent, and the entry are redacted
  together in one pass before the request is built.
- Messages are capped at 2,000 characters, the resent conversation at 12 turns
  and 8,000 characters, and an entry carried as context at 16 KB.
- The composer says where the message is going, that likely secrets are replaced
  first, and that answers may be wrong.

### 🐛 Fixes

- Typing `?` in any field no longer opens the keyboard shortcuts list. The
  shortcut had been registered to fire even while typing, so it interrupted the
  assistant composer, the search box, and the entry form alike. It still works
  from anywhere that is not a field.

### 🧹 Maintenance

- Moved the entry form's own state into `useCommandEditor`, which is also how a
  proposal reaches the form. `App.tsx` stays under the file size limit.
- Added `copyText` for clipboard-only copies of text that is not a saved entry.
- Fixed three explanation tests that had been asserting against copy changed in
  0.8.0's last commit.

### 🧪 Testing

- Added Rust coverage for message and entry size limits, turn-count and size
  bounds on the resent conversation, redaction across the question, history, and
  entry, proposal normalization, deduplication, the proposal ceiling, risk
  precedence in both directions, and the assistant IPC gate.
- Added real cancellation tests: aborting a running task, finishing without
  aborting, reusing a request id, and tracking requests separately.
- Added frontend coverage for the panel's states, the proposal actions, the
  follow-up payload, stopping, retrying, clearing, and discarding the
  conversation when AI is turned off.
- Verified panel geometry, tap targets, label wrapping, and text contrast at
  320, 375, 414, 768, 1280, and 1920 pixels across the dark, light, and
  high-contrast themes.

## 0.8.0 - July 26, 2026

Phase 7A adds Explain. Ask what a saved entry actually does, get one structured
answer that fills a quick summary and a detailed breakdown, and keep it: the
explanation is stored locally, joins search, and marks itself stale the moment
you edit the entry it describes.

### 💡 Explain

- Added an Explanation section to the full entry dialog, shown only when AI is
  enabled and a key is stored. Library cards are untouched.
- One structured response fills both views. Quick is the summary; Detailed adds
  the flag-by-flag breakdown, pipeline stages, side effects, safety review,
  assumptions, and version-sensitive caveats, with no second request.
- Added an optional safer preview command, shown with its own local risk result
  and a Copy action. It is never run and never saved from the panel.
- Every explanation says which model wrote it, when, and that it is
  AI-generated and may be wrong.

### 🛡️ Local authority

- The safety level is the higher of the local verdict and the model's, so a
  model cannot talk a destructive command down.
- Local reasons and model reasons are listed separately and labelled by source.
- A proposed preview is dropped unless the local rules agree it is safer than
  the command it previews, and dropped when it only restates the original.

### 🗃️ Cache and search

- Added an `ai_explanations` table, one row per entry, removed with the entry
  it belongs to.
- Editing an entry keeps its explanation but marks it stale, drops it out of
  search, and offers Refresh.
- Current explanation text is searchable alongside the entry's own title,
  content, description, tags, and collections.
- Added Clear saved explanations to Settings. It works whether AI is on or off,
  because turning AI off hides explanations rather than deleting them.

### 🔒 Privacy

- The entry, its title, description, shell, and language are redacted together
  before the request is built, so a likely secret in any of them is replaced.
- Entries are capped at 16 KB per explanation request and sent with
  `store: false`.
- Cached explanations stay hidden while AI is off, including from search.

### 🏷️ Versioning

- Synced the Rust crate version with the root `package.json`. It had been stuck
  at 0.2.0, which is what the OpenAI `User-Agent` header was reporting.

### 🧪 Testing

- Added Rust coverage for the schema, parsing, list bounds, risk precedence,
  preview rejection, redaction, staleness, cascade deletion, cache clearing,
  search inclusion and exclusion, and the version 2 search rebuild.
- Added frontend coverage for the gated section, the cached read, the Quick and
  Detailed views, labelled safety reasons, the preview copy action, the stale
  banner and refresh, request failure, and the Settings clear action.
- Verified layout, contrast, and touch targets at 320, 375, 414, 768, and
  desktop widths across the dark, light, and high-contrast themes.

## 0.7.0 - July 26, 2026

Phase 6B brings Import back as an AI-assisted workflow. A document is read on
this machine, redacted, and shown to you in full before anything is sent, and
every suggestion that comes back is re-checked by the same local rules the
manual editor uses.

### 📥 AI-assisted import

- Reintroduced Import in the sidebar and command palette, visible only when AI
  is enabled and a key is stored.
- Added structured extraction through the Responses API, so inconsistent
  Markdown, prose, transcripts, and loose cheat sheets produce usable
  candidates with titles, descriptions, tags, kinds, and shells.
- Added an outbound-request step that states the exact size being sent, the
  model handling it, and every likely secret that was replaced, listed by line
  and placeholder rather than by value.
- Kept the existing review screen, duplicate actions, and import writer in
  charge of everything that reaches the library.

### 🛡️ Local authority

- Effective risk is the higher of the local verdict and the model's suggestion,
  so a model cannot talk a destructive command down.
- Model risk reasons and uncertainty notes are shown separately and labelled as
  model output that may be wrong.
- Locally detected terminal output stays unticked even when the model disagrees,
  and the model can still flag output the local rules missed.
- Structural kinds (scripts, sequences) come from the local classifier; the
  model only decides between command, snippet, and reference.
- Editing, splitting, or merging a candidate clears the model's notes for it,
  because they no longer describe what is on screen.

### 🔒 Privacy

- Documents are redacted before serialization, capped at 64 KB per request, and
  sent with `store: false`.
- A refusal, an incomplete run, or a malformed response opens no candidates and
  writes nothing.

### 🧱 Internals

- Split the OpenAI client into a reusable structured-request path with
  per-request budgets and timeouts, shared transport error mapping, and
  versioned prompts and schemas in one place.
- The database lock is released before every awaited network call.

### 🧪 Testing

- Added Rust coverage for schema strictness, request bounds, extraction
  sanitizing, item ceilings, risk precedence, output authority, duplicate
  detection, disclosure line numbers, and the disabled-AI gate.
- Added frontend coverage for the disclosure copy, redaction grouping, the
  full paste-to-review flow, model-note labelling, and the gated sidebar entry.
- Fixed an AI Settings test that expected credential-status copy the component
  never shipped.

### 🐛 Fixes

- Raised the output budget for both import and Test connection. `max_output_tokens`
  caps reasoning tokens as well as visible output, so a small reasoning model
  could spend the whole budget thinking and never finish, even on a 2 KB cheat
  sheet. Import now starts at 25,000 tokens and scales to 64,000; the connection
  test gets 25,000. The cap is billed on tokens generated, not tokens reserved.
- Reported the real reason a response was incomplete. An exhausted output budget
  no longer claims the document was too long, and a content-filter stop says so.
- Said plainly when a model refuses an output limit that large, instead of
  reporting it as a rejected structured-output request.
- Rewrote the extraction prompt so it extracts commands instead of transcribing
  the document. One command per entry by default, a trailing explanatory comment
  becomes the title, and the document's own formatting (column padding, Markdown
  escaping, list markers, fences) is stripped rather than preserved.
- Repaired column padding that comes back as literal `\t` text, while leaving a
  deliberate `\t` in a command alone.

### 🏷️ Versioning

- Bumped the app from 0.6.1 to 0.7.0 for the new import workflow.

## 0.6.1 - July 26, 2026

API key entry now remains available when the initial credential-manager check
cannot confirm the store, so a temporary or locked read failure no longer
blocks setup.

### 🐛 AI settings

- Kept the Add key control available when secure storage cannot be verified
  during the preliminary status check.
- Made Save retry the operating system credential manager and restore the
  normal stored state after a successful write.
- Added a specific unlock instruction for locked credential stores while
  keeping platform error details out of the frontend.
- Preserved the no-plaintext-fallback rule.

### 🧪 Testing

- Added frontend coverage for entering and saving a key after an unavailable
  status result.
- Added Rust coverage for sanitized locked-store errors.
- Confirmed the local Linux Secret Service is reachable without reading or
  writing an API key.

### 🏷️ Versioning

- Bumped the app from 0.6.0 to 0.6.1 for the key-entry fix.

## 0.6.0 - July 26, 2026

Phase 6A adds the optional OpenAI foundation without changing the local command
library. AI stays off by default, and Import, Explain, and assistant entry
points remain hidden.

### ⚙️ AI settings

- Added the off-by-default AI switch, `gpt-5.6-luna` default, Rust-provided
  GPT-5-and-newer model list, and unrestricted custom model field.
- Added one-time API key entry with stored, unavailable, replacement, and
  confirmed removal states.
- Added a connection test that checks the saved model with the Responses API
  and strict structured output instead of accepting a generic text response.

### 🔐 Privacy and credentials

- Added native OS credential storage through Keychain, Credential Manager, or
  Secret Service with no SQLite, frontend storage, logging, or plaintext
  fallback.
- Kept stored keys behind Rust. React receives only stored or not stored status
  and clears a newly entered key immediately after a successful save.
- Added local redaction for likely private keys, provider tokens, bearer
  tokens, credential assignments, and credentials embedded in URLs.

### 🧠 OpenAI boundary

- Added one reusable Rust HTTP client with a fixed HTTPS endpoint, redirects
  disabled, bounded timeouts, bounded output, `store: false`, and distinct
  authentication, model, quota, network, refusal, and response errors.
- Added a backend feature gate so network-backed AI commands fail while AI is
  disabled, even if the frontend command is invoked directly.
- Kept the SQLite mutex out of the network path and kept every future AI
  workflow behind the same Rust-owned service.

### 🧪 Testing

- Added request-serialization tests proving the API key is not in the body and
  provider storage is disabled.
- Added tests for old settings, custom models, error kinds, structured response
  validation, secret redaction, key lifecycle UI, and the disabled AI state.
- No automated test or implementation check sent a live OpenAI request.

### 🏷️ Versioning

- Bumped the app from 0.5.2 to 0.6.0 for the new user-facing AI settings and
  credential workflow.

## 0.5.2 - July 26, 2026

The optional AI work now has one implementation plan covering configuration,
privacy boundaries, AI-assisted Import, explanations, and the command
assistant before development begins.

### 🗺️ Planning

- Added a phased AI implementation plan with architecture, request flows,
  data ownership, tests, manual verification, and completion gates.
- Split the work into AI foundation, assisted Import, explanation caching,
  command assistance, error analysis, and shell conversion.
- Kept the existing command library fully local and independent from the
  planned AI features.

### 🔐 Privacy and configuration

- Set `gpt-5.6-luna` as the planned default while keeping a curated
  GPT-5-and-newer dropdown and unrestricted custom model field.
- Defined the one-time API key entry flow, OS credential storage, backend-only
  retrieval, explicit replacement and removal, and no plaintext fallback.
- Defined local secret redaction, visible outbound-request context,
  `store: false`, memory-only chat history, and backend feature gating.
- Preserved local risk rules as authoritative for imports, explanations, and
  generated command proposals.

### 🏷️ Versioning

- Bumped the documentation-planning release from 0.5.1 to 0.5.2.

## 0.5.1 - July 26, 2026

The Linux AppImage can now start reliably on CachyOS systems affected by
incompatible GIO crypto-module loading.

### 🐛 AppImage compatibility

- Added a Linux AppImage-only startup guard that disables dynamically loaded
  GIO modules before Tauri initializes.
- Gated the workaround on the AppImage runtime's `APPIMAGE` variable so normal
  development, `.deb`, `.rpm`, Windows, macOS, and mobile builds are unchanged.
- Disabled both `GIO_MODULE_DIR` and `GIO_EXTRA_MODULES`; changing only one
  still allows the generated AppImage hooks to load the incompatible module.

### 🧪 Testing

- Added a focused Rust test proving the compatibility path stays disabled
  outside an AppImage and activates when the runtime marker is present.
- Built and launch-tested the exact 0.5.1 AppImage on CachyOS with
  `NO_STRIP=true`.

### 🏷️ Versioning

- Bumped the app from 0.5.0 to 0.5.1.

## 0.5.0 - July 25, 2026

Phase 9 completes the practical non-AI app: faster navigation, built-in help,
safe exports and backups, desktop integration, and guarded database recovery.

### 🧭 Navigation and help

- Added a searchable command palette on `Ctrl/Cmd + K` for navigation, entry
  creation, library actions, export, backup, Help, shortcuts, and Settings.
- Kept `/` as the direct shortcut for focusing the main library search.
- Added a mobile-friendly Help guide beside Keyboard shortcuts and Settings.
- Documented the exact `ssh {{user}}@{{host}}` placeholder workflow, temporary
  value handling, clipboard behavior, and the fact that commands never run
  inside the app.

### 💾 Data safety

- Added a complete, readable Markdown export with metadata, notes, source
  links, and safe code fences.
- Added a consistent SQLite backup action that checkpoints pending WAL data and
  refuses to overwrite the active library.
- Added startup recovery for unreadable databases. Command Center offers to
  preserve the damaged database and its sidecars before creating a fresh
  library; it never silently deletes the old data.

### 🖥️ Desktop integration

- Added an opt-in launch-at-startup preference backed by the operating system.
- Added an opt-in close-to-tray preference, a tray action for restoring the
  window, and an explicit Quit action.
- Kept both desktop behaviors off by default and made older saved settings load
  with those safe defaults.

### 🎨 Branding and polish

- Added the Command Center logo to the sidebar and removed the redundant
  sidebar version badge.
- Regenerated desktop, Android, and iOS application icons from
  `public/icon.png`.
- Added the Apache License 2.0 and recorded `Apache-2.0` in package metadata.
- Verified the new surfaces at desktop and 320px, 375px, and 414px mobile
  widths with keyboard, touch, focus, and overflow checks.

### 🧪 Testing

- Added real SQLite tests for Markdown export, database backup, and unreadable
  database preservation.
- Added frontend tests for Help, command-palette filtering and keyboard
  behavior, file-dialog flows, settings persistence, and navigation.
- The full suite now contains 108 frontend tests and 111 Rust tests.

### 🏷️ Versioning

- Bumped the app from 0.4.2 to 0.5.0.

## 0.4.2 - July 25, 2026

Cards are now the starting library view, and keyboard help has a permanent,
mobile-friendly home in the sidebar.

### 🎨 Library view

- Made Cards the default for fresh installs and settings without a saved view.
- Preserved Compact for anyone who has explicitly saved it as their preference.

### ⌨️ Navigation

- Moved Keyboard shortcuts from the floating library footer into the sidebar,
  directly above Settings.
- Opening shortcut help from the mobile sidebar now closes the navigation
  drawer before showing the dialog.

### 🧹 Maintenance

- Removed a stale Create Collection prop and test assertion left behind when
  that sidebar action was retired.

### 🗺️ Roadmap

- Marked Phase 8 complete at the existing basic-placeholder scope.
- Deferred parameter metadata such as defaults, validation, masking, and
  remembered values until normal use shows a need for them.

### 🏷️ Versioning

- Bumped the app from 0.4.1 to 0.4.2.

## 0.4.1 - July 25, 2026

Long commands and scripts are easier to inspect without letting one entry take
over the library.

### 📜 Command viewing

- Command cards now stay at a consistent size in both Compact and Cards views.
- Long commands and scripts scroll inside their bounded previews instead of
  stretching the surrounding layout.
- Clicking an entry title or code preview opens a wide full-entry dialog with
  the complete content, template fields, risk explanation, notes, metadata,
  and labelled actions.
- The full-entry dialog becomes a safe-area-aware full-screen view on mobile.

### 🧹 Card cleanup

- Removed copy counts and last-copied timestamps from card presentation while
  retaining internal copy history for Recent and sorting.
- Replaced Copy, Edit, and Delete card labels with icon-only actions that keep
  accessible names and hover tooltips.
- Replaced visible Caution and Destructive badges on cards with compact yellow
  and red icons in the upper-right status position. The full explanation
  remains available in the tooltip and full-entry dialog.

### 🗺️ Roadmap

- Renamed Phase 8 to Parameterized commands and scripts and documented the
  current placeholder behavior, planned field settings, secret handling, value
  persistence, and the unresolved semantics of optional command fragments.

### 🏷️ Versioning

- Bumped the app from 0.4.0 to 0.4.1.

## 0.4.0 - July 25, 2026

Collections are easier to find, use, and change without forcing every command
into a default bucket.

### 🗂️ Collections

- Added separate Create and Manage actions beside the Collections heading in
  the sidebar.
- Added collection-specific Rename, Delete, and Manage all actions to the
  active collection page.
- Collection deletion now asks for confirmation and makes it clear that saved
  commands stay in the library.
- Fresh libraries remain empty until the user creates a collection. Command
  Center does not seed starter collections or tags.

### ✏️ Command editor

- Moved Collections directly below Tags in Add Command and Edit Command.
- Replaced the old checkbox grid with a compact dropdown and removable selected
  chips while preserving support for multiple collections.
- Added an inline Create new collection path without leaving the command form.
- New commands are uncategorized unless the user chooses a collection or opens
  Add Command from a collection page.

### ⚙️ Settings

- Removed the Default collection preference from the frontend and backend
  settings model.
- Existing settings JSON remains compatible; the retired field is ignored.

### ♿ Accessibility

- Collection actions have descriptive labels, Escape handling, visible focus,
  and full keyboard and touch access.
- Dialogs can now focus the intended first field when opened directly in Create
  or Rename mode.

### 🏷️ Versioning

- Bumped the app from 0.3.0 to 0.4.0.

## 0.3.0 - July 25, 2026

A tighter command library with two practical layouts and a darker
high-contrast theme. Quick Add has been removed, and the current local Import
screen is paused until it can return with AI-assisted handling for inconsistent
source documents.

### 📚 Library views

- Compact list is the new default. Each command uses two dense scan lines on
  desktop, with its title, command, metadata, usage, and actions kept close
  together.
- Cards can be selected in Settings. The grid adapts from one column on mobile
  to four on large fullscreen displays, and expanded entries use the full row.
- Both layouts retain expansion, template fields, risk explanations, copy,
  edit, favorite, delete, keyboard navigation, and mobile-friendly controls.

### 🎨 High contrast

- Added a selectable High contrast dark theme with an almost-black chromatic
  background, darker surfaces, stronger borders, and crisp text.
- The existing Dark, Light, and Match the system options remain available.

### 🧹 Focus

- Removed Quick Add completely, including its separate window, global
  shortcut, settings, IPC commands, styles, build entry, capability, and
  dependency.
- Removed the Import entry point from the sidebar. Its local parser, review
  workflow, backend, and tests remain in the codebase for a later AI-backed
  reintroduction.
- Existing saved settings load safely: removed Quick Add fields are ignored,
  and the new library-view preference defaults to Compact without resetting
  other preferences.

### 🏷️ Versioning

- Bumped the app from 0.2.0 to 0.3.0.

## 0.2.0 - July 25, 2026

Markdown import. Point Command Center at a cheat sheet, README, or your own
notes and it pulls the commands out. Parsing happens entirely on this machine;
there is still no network code anywhere in the app.

### 📥 Import

- New Import screen, reachable from the sidebar. Three ways in: drop a file on
  the window, pick one with the file dialog, or paste content straight in.
- Reads Markdown, plain text, README files, and notes: fenced code blocks
  (with or without a language), indented blocks, `$` and `PS>` prompt lines
  outside a fence, and standalone inline code spans.
- Headings become titles when a heading covers exactly one block. The paragraph
  above a block becomes its description.
- Tags are suggested from the tool being run (`pacman`, `systemctl`,
  `grub-install`) plus short topic-style headings. Sentence headings are left
  alone instead of being shredded into filler words.
- The document's first heading, or the file name, is suggested as a collection.

### 🧹 Knowing what not to import

- Terminal output is detected and left unticked, with the reason shown: a
  known output header, column layout, an explicit plain-text fence, or too few
  lines that read like commands.
- A pasted shell session keeps only the prompted commands and reports how many
  output lines were dropped. Backslash continuations stay attached to their
  command.
- Markdown lists are never mistaken for indented code.

### 🔁 Duplicates

- Every candidate is checked against the library using the same normalized
  content hash the app already used, so prompt prefixes and stray whitespace
  cannot hide a match.
- Four choices per duplicate: skip (the default), merge, replace, or keep both.
  Merge keeps the saved command and folds in new tags, collections, and any
  description or notes it was missing.
- Commands repeated within one document are flagged and only ticked once.

### ✏️ Review screen

- Every candidate is editable before anything is saved: title, command,
  description, type, and tags.
- Split a multi-line block into one entry per line, merge a block into the one
  above it, or remove it entirely. Splitting and merging re-run classification,
  so risk, type, and duplicate status stay correct.
- Bulk select all or none, and assign one collection to everything at once.
- Import reports what happened: added, replaced, merged, skipped, and anything
  that failed, with the reason. One bad entry no longer stops the batch.

### 📖 Documentation

- Documented both providers in the README: what each one is, how to set it up,
  what leaves the machine, and that Codex has to be installed separately
  because the executable is around 250 MB.
- Said plainly that a connected ChatGPT account follows that plan's usage
  limits and is not free API access.

### 🧹 Interface text

- The Codex program path field now appears only when it is needed: when Codex
  was not found, is too old, cannot be used, or when a path is already saved.
  A working install shows its version and nothing to configure.
- Moved the Codex explanations out of the panel and into Help, which now
  describes both providers, what each one stores, and why signing in to
  Command Center does not change the account the Codex CLI uses.

### 🧪 Tests

- 105 Rust tests (up from 61) and 73 frontend tests (up from 44), including a
  full realistic cheat sheet exercised end to end through the real parser.

## 0.1.0 - July 25, 2026

First working version. The command library is complete and usable on its own;
import and AI features are not built yet.

### 🗄️ Foundation

- Tauri 2 desktop shell with a React 19 + TypeScript + Vite frontend.
- SQLite storage through `rusqlite` with bundled SQLite, a versioned migration
  runner driven by `PRAGMA user_version`, WAL journaling, and enforced foreign
  keys.
- One library file per install in the platform app-data directory. The path is
  shown in Settings → About.

### 📚 Command library

- Create, edit, delete, and copy entries. Five entry kinds: command, script,
  sequence, snippet, and reference.
- Optional fields (shell, operating system, language, working directory, source
  URL, notes, risk override, collections, favorite) live behind an
  "Add extra details" disclosure so the default form stays four fields.
- A blank title falls back to the first meaningful line of the command.
- Copying an entry increments its counter, stamps the copy time, and writes a
  usage history row.

### 🔍 Search and organization

- SQLite FTS5 index over title, command text, description, notes, tags, and
  collection names, rebuilt on every write so results never go stale.
- Query building quotes every token, so punctuation-heavy searches like `rm -rf`
  cannot become syntax errors, and adds prefix matching for results while typing.
- Scopes for All, Favorites, Recent, Scripts, a collection, or a tag; filters by
  entry type; sorting by updated, created, title, copy count, or last copied.
- Collections with create, rename, and delete. Deleting a collection keeps its
  entries. Renaming one reindexes its members.
- Tags are normalized to lowercase, deduplicated, and pruned when nothing uses
  them any more.

### ⚠️ Risk labels

- Offline rules classify entries as Safe, Caution, or Destructive and list the
  reasons. Destructive covers recursive force deletes, `mkfs`, `dd of=`,
  `shred`, `git reset --hard`, force pushes, `git clean`, dropped tables,
  `find -delete`, piping a download into a shell, fork bombs, and raw writes to
  disk devices.
- Local rules always win over anything else; a manual override per entry is
  respected and only shows warnings at or below the chosen level.

### ⚡ Quick Add

- Global shortcut (`Ctrl/Cmd + Shift + Space` by default) opens a compact capture
  window from anywhere, configurable in Settings.
- Paste, `Ctrl + Enter` to save, `Esc` to dismiss. Optional title, tags, and
  collection.
- Warns when the same command already exists, comparing normalized content so
  prompt prefixes and trailing whitespace do not hide a duplicate.

### 🧩 Templates

- `{{placeholder}}` detection with a fill-in form in the expanded card. Copying
  renders the filled version; blanks keep their placeholder rather than silently
  becoming empty.

### ⌨️ Keyboard and accessibility

- `Ctrl/Cmd + K` or `/` to search, `N` to add, arrows to walk the list, `Enter`
  to expand, `Esc` to close or clear, `?` for the shortcut list.
- Focus-visible rings everywhere, a skip link, focus-trapped dialogs that return
  focus on close, `aria-pressed` favorites, `aria-expanded` cards, live regions
  for toasts, and risk shown as icon plus word rather than colour alone.

### 🎨 Interface

- Dark charcoal default with a light theme and a "match the system" option.
- Token-driven OKLCH colour, IBM Plex Sans paired with JetBrains Mono, both
  bundled locally so the app never asks the network for a font.
- Responsive down to 320px: the sidebar becomes a slide-over drawer and the
  filter row wraps.

### 📖 Documentation

- Documented both providers in the README: what each one is, how to set it up,
  what leaves the machine, and that Codex has to be installed separately
  because the executable is around 250 MB.
- Said plainly that a connected ChatGPT account follows that plan's usage
  limits and is not free API access.

### 🧹 Interface text

- The Codex program path field now appears only when it is needed: when Codex
  was not found, is too old, cannot be used, or when a path is already saved.
  A working install shows its version and nothing to configure.
- Moved the Codex explanations out of the panel and into Help, which now
  describes both providers, what each one stores, and why signing in to
  Command Center does not change the account the Codex CLI uses.

### 🧪 Tests

- 61 Rust tests covering migrations, CRUD, search behaviour, index maintenance,
  normalization, duplicate detection, risk rules, settings, and cascade cleanup,
  all against real SQLite databases.
- 44 frontend tests covering formatting helpers, hotkey matching, error
  normalization, card behaviour, form validation, and dialog focus management.
