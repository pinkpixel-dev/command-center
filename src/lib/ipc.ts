import { invoke } from "@tauri-apps/api/core";

import type {
  ImportItem,
  ImportPreview,
  ImportSummary,
  SnippetAnalysis,
} from "./import";
import type {
  AppErrorPayload,
  AppSettings,
  Collection,
  CollectionInput,
  CommandEntry,
  CommandInput,
  LibraryStats,
  ListQuery,
  Tag,
} from "./types";

/** Event names the Rust side emits. */
export const LIBRARY_CHANGED = "library-changed";
export const SETTINGS_CHANGED = "settings-changed";

/**
 * Rejected invokes arrive as `{ kind, message }`. Anything else (a panic, a
 * missing command) is wrapped so callers only ever handle one shape.
 */
export function toAppError(error: unknown): AppErrorPayload {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as AppErrorPayload).message === "string"
  ) {
    const payload = error as AppErrorPayload;
    return { kind: payload.kind ?? "runtime", message: payload.message };
  }
  if (typeof error === "string") {
    return { kind: "runtime", message: error };
  }
  return { kind: "runtime", message: "Something went wrong. Check the app logs." };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

export const api = {
  listCommands: (filter: ListQuery = {}) => call<CommandEntry[]>("list_commands", { filter }),
  getCommand: (id: number) => call<CommandEntry>("get_command", { id }),
  createCommand: (input: CommandInput) => call<CommandEntry>("create_command", { input }),
  updateCommand: (id: number, input: CommandInput) =>
    call<CommandEntry>("update_command", { id, input }),
  deleteCommand: (id: number) => call<void>("delete_command", { id }),
  toggleFavorite: (id: number) => call<boolean>("toggle_favorite", { id }),
  recordCopy: (id: number) => call<CommandEntry>("record_copy", { id }),
  findDuplicate: (content: string) => call<CommandEntry | null>("find_duplicate", { content }),
  libraryStats: () => call<LibraryStats>("library_stats"),

  listTags: () => call<Tag[]>("list_tags"),
  listCollections: () => call<Collection[]>("list_collections"),
  createCollection: (input: CollectionInput) => call<Collection[]>("create_collection", { input }),
  updateCollection: (id: number, input: CollectionInput) =>
    call<Collection[]>("update_collection", { id, input }),
  deleteCollection: (id: number) => call<Collection[]>("delete_collection", { id }),

  previewImportText: (content: string, sourceName?: string) =>
    call<ImportPreview>("preview_import_text", { content, sourceName: sourceName ?? null }),
  previewImportFile: (path: string) => call<ImportPreview>("preview_import_file", { path }),
  analyzeSnippet: (content: string) => call<SnippetAnalysis>("analyze_snippet", { content }),
  importCommands: (items: ImportItem[]) => call<ImportSummary>("import_commands", { items }),

  getSettings: () => call<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    call<AppSettings>("save_settings", { settingsInput: settings }),
  libraryLocation: () => call<string>("library_location"),
};
