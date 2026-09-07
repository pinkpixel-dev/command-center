import type { ImportDocument } from "./ai-import";
import type { Collection } from "./types";

// Resolved at build time: `ipc-tauri.ts` for the desktop app, `ipc-http.ts`
// for the web bundle. Neither implementation ends up in the other's bundle.
import { platform as implementation } from "@platform";

/**
 * Reads a document the user chose or dropped. The reading is deferred so the
 * caller that owns the error handling is the one that runs it.
 */
export type ImportDocumentReader = () => Promise<ImportDocument>;

/** What a drop target needs to know. Drops are watched at the window level. */
export interface DropHandlers {
  onOver: () => void;
  onLeave: () => void;
  /** Called with null when the drop carried nothing readable. */
  onDrop: (read: ImportDocumentReader | null) => void;
}

/**
 * Everything the app does that is not a backend command with a JSON answer.
 * The desktop app goes through Tauri; the web bundle goes through the browser
 * and the server.
 */
export interface Platform {
  /** Runs a backend command. Rejects with `{ kind, message }`. */
  call: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  /** Subscribes to a backend event. Returns the unsubscribe. */
  subscribe: <T>(event: string, handler: (payload: T) => void) => () => void;
  /** Opens a link outside the app. */
  openUrl: (url: string) => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;

  /** Opens the file picker. Resolves to null when the user backs out. */
  chooseImportDocument: () => Promise<ImportDocumentReader | null>;
  watchFileDrops: (handlers: DropHandlers) => () => void;

  // Each one resolves to where the file went, or null when the user backed
  // out. Only the caller's "did it happen" check reads the value.
  exportLibraryMarkdown: () => Promise<string | null>;
  exportCollectionMarkdown: (collection: Collection) => Promise<string | null>;
  backupLibrary: () => Promise<string | null>;
}

export const platform: Platform = implementation;
