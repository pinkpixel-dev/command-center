import { save } from "@tauri-apps/plugin-dialog";

import { api } from "./ipc";

function dateStamp(now = new Date()): string {
  return now.toISOString().slice(0, 10);
}

export async function exportLibraryMarkdown(): Promise<string | null> {
  const destination = await save({
    title: "Export Command Center library",
    defaultPath: `command-center-${dateStamp()}.md`,
    filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
  });
  if (!destination) return null;
  return api.exportLibraryMarkdown(destination);
}

export async function backupLibraryDatabase(): Promise<string | null> {
  const destination = await save({
    title: "Back up Command Center library",
    defaultPath: `command-center-backup-${dateStamp()}.db`,
    filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] }],
  });
  if (!destination) return null;
  return api.backupLibrary(destination);
}
