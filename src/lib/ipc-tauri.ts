import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

import type { ImportDocument } from "./ai-import";
import type { DropHandlers, ImportDocumentReader, Platform } from "./platform";
import type { Collection } from "./types";

/**
 * The desktop half of the platform boundary. Every call here is one Tauri API
 * away from the OS: real dialogs, real paths, the system browser.
 */

const FILE_FILTERS = [
  {
    name: "Documents",
    extensions: ["md", "markdown", "mdx", "txt", "text", "rst", "adoc", "org"],
  },
];

const MARKDOWN_FILTERS = [{ name: "Markdown", extensions: ["md", "markdown"] }];
const DATABASE_FILTERS = [
  { name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] },
];

function dateStamp(now = new Date()): string {
  return now.toISOString().slice(0, 10);
}

/** Turns a collection name into the middle of a suggested filename. */
function fileSafeName(name: string): string {
  return (
    name
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "collection"
  );
}

function readDocument(path: string): ImportDocumentReader {
  return () => invoke<ImportDocument>("read_import_document", { path });
}

/**
 * Bridges Tauri's async subscription to the synchronous unsubscribe the
 * interface promises. Unsubscribing before the listener has registered is
 * remembered, so a component that unmounts immediately does not leak one.
 */
function bridge(pending: Promise<UnlistenFn>): () => void {
  let stop: UnlistenFn | null = null;
  let cancelled = false;

  void pending.then((unlisten) => {
    if (cancelled) {
      unlisten();
      return;
    }
    stop = unlisten;
  });

  return () => {
    cancelled = true;
    stop?.();
    stop = null;
  };
}

export const platform: Platform = {
  // Nothing to sign in to. The library is a file this user already owns.
  auth: null,

  call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return invoke<T>(command, args);
  },

  subscribe<T>(event: string, handler: (payload: T) => void): () => void {
    return bridge(listen<T>(event, (received) => handler(received.payload)));
  },

  openUrl(url: string): Promise<void> {
    return openUrl(url);
  },

  async copyToClipboard(text: string): Promise<void> {
    await writeText(text);
  },

  async chooseImportDocument(): Promise<ImportDocumentReader | null> {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: FILE_FILTERS,
    });
    return typeof selected === "string" ? readDocument(selected) : null;
  },

  // Tauri reports drops at the window level, so the whole window is the drop
  // target whichever element asked to watch.
  watchFileDrops(handlers: DropHandlers): () => void {
    return bridge(
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          handlers.onOver();
          return;
        }
        if (event.payload.type === "drop") {
          handlers.onLeave();
          const [first] = event.payload.paths;
          handlers.onDrop(first ? readDocument(first) : null);
          return;
        }
        handlers.onLeave();
      }),
    );
  },

  async exportLibraryMarkdown(): Promise<string | null> {
    const destination = await save({
      title: "Export Command Center library",
      defaultPath: `command-center-${dateStamp()}.md`,
      filters: MARKDOWN_FILTERS,
    });
    if (!destination) return null;
    return invoke<string>("export_library_markdown", { destination });
  },

  async exportCollectionMarkdown(collection: Collection): Promise<string | null> {
    const destination = await save({
      title: `Export ${collection.name}`,
      defaultPath: `command-center-${fileSafeName(collection.name)}-${dateStamp()}.md`,
      filters: MARKDOWN_FILTERS,
    });
    if (!destination) return null;
    return invoke<string>("export_collection_markdown", {
      collectionId: collection.id,
      destination,
    });
  },

  async backupLibrary(): Promise<string | null> {
    const destination = await save({
      title: "Back up Command Center library",
      defaultPath: `command-center-backup-${dateStamp()}.db`,
      filters: DATABASE_FILTERS,
    });
    if (!destination) return null;
    return invoke<string>("backup_library", { destination });
  },
};
