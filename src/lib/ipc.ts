import { invoke } from "@tauri-apps/api/core";

import type { AiImportPlan, ImportDocument } from "./ai-import";
import type {
  ImportItem,
  ImportPreview,
  ImportSummary,
  SnippetAnalysis,
} from "./import";
import type { OutboundPlan } from "./outbound";
import type {
  AppErrorPayload,
  AiConnectionResult,
  AiKeyStatus,
  AiStatus,
  AppSettings,
  AssistantAsk,
  AssistantReply,
  Collection,
  CollectionInput,
  CommandEntry,
  CommandInput,
  ErrorAnalysis,
  ExplanationView,
  LibraryStats,
  ListQuery,
  ShellConversion,
  ShellOption,
  Tag,
  TargetShell,
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
  deleteCommands: (commandIds: number[]) =>
    call<void>("delete_commands", { commandIds }),
  addCommandsToCollection: (commandIds: number[], collectionId: number) =>
    call<void>("add_commands_to_collection", { commandIds, collectionId }),
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

  readImportDocument: (path: string) => call<ImportDocument>("read_import_document", { path }),
  prepareAiImport: (content: string) => call<AiImportPlan>("prepare_ai_import", { content }),
  runAiImport: (content: string, sourceName?: string | null) =>
    call<ImportPreview>("run_ai_import", { content, sourceName: sourceName ?? null }),

  previewImportText: (content: string, sourceName?: string) =>
    call<ImportPreview>("preview_import_text", { content, sourceName: sourceName ?? null }),
  previewImportFile: (path: string) => call<ImportPreview>("preview_import_file", { path }),
  analyzeSnippet: (content: string) => call<SnippetAnalysis>("analyze_snippet", { content }),
  importCommands: (items: ImportItem[]) => call<ImportSummary>("import_commands", { items }),

  getSettings: () => call<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    call<AppSettings>("save_settings", { settingsInput: settings }),
  getAiStatus: () => call<AiStatus>("get_ai_status"),
  saveAiKey: (apiKey: string) => call<AiKeyStatus>("save_ai_key", { apiKey }),
  removeAiKey: () => call<AiKeyStatus>("remove_ai_key"),
  testAiConnection: () => call<AiConnectionResult>("test_ai_connection"),
  askAssistant: (request: AssistantAsk) => call<AssistantReply>("ask_assistant", { request }),
  cancelAssistantRequest: (requestId: number) =>
    call<boolean>("cancel_assistant_request", { requestId }),
  getCommandExplanation: (commandId: number) =>
    call<ExplanationView | null>("get_command_explanation", { commandId }),
  explainCommand: (commandId: number) =>
    call<ExplanationView>("explain_command", { commandId }),
  clearAiExplanations: () => call<number>("clear_ai_explanations"),

  prepareErrorAnalysis: (output: string) =>
    call<OutboundPlan>("prepare_error_analysis", { output }),
  analyzeTerminalError: (requestId: number, output: string) =>
    call<ErrorAnalysis>("analyze_terminal_error", { requestId, output }),

  conversionShells: () => call<ShellOption[]>("conversion_shells"),
  convertCommandShell: (requestId: number, commandId: number, targetShell: TargetShell) =>
    call<ShellConversion>("convert_command_shell", { requestId, commandId, targetShell }),

  libraryLocation: () => call<string>("library_location"),
  exportLibraryMarkdown: (destination: string) =>
    call<string>("export_library_markdown", { destination }),
  exportCollectionMarkdown: (collectionId: number, destination: string) =>
    call<string>("export_collection_markdown", { collectionId, destination }),
  backupLibrary: (destination: string) => call<string>("backup_library", { destination }),
};
