import { LIBRARY_CHANGED } from "./events";
import type { ImportDocument } from "./ai-import";
import type { DropHandlers, ImportDocumentReader, Platform } from "./platform";
import type { AppErrorPayload, Collection } from "./types";

/**
 * The web half of the platform boundary. Commands are `POST /api/<name>` with
 * the same argument object the desktop app sends to Tauri, notifications
 * arrive as server-sent events, and the two file edges become an upload and a
 * download.
 *
 * The server is always the origin serving the page, so every path is relative.
 */

const BASE = "/api";

/** What the file picker offers, matching what the backend agrees to read. */
const ACCEPT = ".md,.markdown,.mdx,.txt,.text,.rst,.adoc,.org";

/**
 * A blob URL has to outlive the click that saves it. Revoking on the next tick
 * is enough in Chrome and Firefox but has raced in Safari, so this waits long
 * enough to stop being interesting.
 */
const REVOKE_DELAY_MS = 60_000;

async function post(command: string, args?: Record<string, unknown>): Promise<Response> {
  try {
    return await fetch(`${BASE}/${command}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args ?? {}),
    });
  } catch {
    throw unreachable();
  }
}

function unreachable(): AppErrorPayload {
  return {
    kind: "network",
    message: "Could not reach the Command Center server. Check that it is still running.",
  };
}

/**
 * Turns a failed response into the `{ kind, message }` shape the app already
 * handles. A body that is not the expected error falls back to the status,
 * which is what a proxy in the way would produce.
 */
async function failure(response: Response): Promise<AppErrorPayload> {
  try {
    const body: unknown = await response.json();
    if (
      typeof body === "object" &&
      body !== null &&
      typeof (body as AppErrorPayload).message === "string"
    ) {
      const payload = body as AppErrorPayload;
      return { kind: payload.kind ?? "runtime", message: payload.message };
    }
  } catch {
    // Not JSON. The status below is all there is to report.
  }
  return {
    kind: "runtime",
    message: `The server answered ${response.status}. Check the server logs.`,
  };
}

async function json<T>(response: Response): Promise<T> {
  if (!response.ok) throw await failure(response);
  const text = await response.text();
  // Commands that return nothing send `null`, but an empty body is treated the
  // same rather than throwing a parse error on the way past.
  return (text.length === 0 ? undefined : JSON.parse(text)) as T;
}

// --- notifications -------------------------------------------------------

type Handler = (payload: unknown) => void;

const listeners = new Map<string, Set<Handler>>();
let feed: EventSource | null = null;
let everConnected = false;

function deliver(event: string, payload: unknown): void {
  for (const handler of listeners.get(event) ?? []) handler(payload);
}

/**
 * One connection for the whole page, opened on the first subscription and left
 * open. EventSource reconnects on its own, and closing it when the last React
 * component unmounts would only mean reopening it on the next render.
 */
function connect(): void {
  if (feed) return;

  const source = new EventSource(`${BASE}/events`);

  source.addEventListener("open", () => {
    // A reconnection means changes may have happened while the browser was
    // away, so the library is reloaded rather than trusted. Settings are
    // picked up the next time they are saved or the page is loaded.
    if (everConnected) deliver(LIBRARY_CHANGED, null);
    everConnected = true;
  });

  for (const event of listeners.keys()) attach(source, event);
  feed = source;
}

function attach(source: EventSource, event: string): void {
  source.addEventListener(event, (message: MessageEvent<string>) => {
    let payload: unknown = null;
    try {
      payload = JSON.parse(message.data);
    } catch {
      // A payload that will not parse still means the thing it names changed.
    }
    deliver(event, payload);
  });
}

// --- files ---------------------------------------------------------------

function pickFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ACCEPT;
    input.addEventListener("change", () => resolve(input.files?.[0] ?? null), { once: true });
    // Backing out of the picker fires this in current browsers. Without it the
    // promise would simply never settle, which nothing is waiting on.
    input.addEventListener("cancel", () => resolve(null), { once: true });
    input.click();
  });
}

/** Uploads the file and reads back what the backend made of it. */
function readDocument(file: File): ImportDocumentReader {
  return async () => {
    const form = new FormData();
    form.append("file", file, file.name);

    let response: Response;
    try {
      response = await fetch(`${BASE}/read_import_document`, { method: "POST", body: form });
    } catch {
      throw unreachable();
    }
    return json<ImportDocument>(response);
  };
}

/** True only for a drag carrying files, so dropping text into a field still works. */
function carriesFiles(event: DragEvent): boolean {
  return Array.from(event.dataTransfer?.types ?? []).includes("Files");
}

/**
 * Asks the server for a file and hands it to the browser to save. Returns the
 * name it was saved under.
 */
async function saveDownload(command: string, args?: Record<string, unknown>): Promise<string> {
  const response = await post(command, args);
  if (!response.ok) throw await failure(response);

  const blob = await response.blob();
  const name = filenameFrom(response.headers.get("Content-Disposition")) ?? "command-center-export";
  const url = URL.createObjectURL(blob);

  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.rel = "noopener";
  document.body.appendChild(link);
  link.click();
  link.remove();

  window.setTimeout(() => URL.revokeObjectURL(url), REVOKE_DELAY_MS);
  return name;
}

/** Reads the filename out of `attachment; filename="…"`. */
export function filenameFrom(header: string | null): string | null {
  if (!header) return null;
  const quoted = /filename="([^"]+)"/.exec(header);
  if (quoted) return quoted[1];
  const bare = /filename=([^;]+)/.exec(header);
  return bare ? bare[1].trim() : null;
}

export const platform: Platform = {
  async call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return json<T>(await post(command, args));
  },

  subscribe<T>(event: string, handler: (payload: T) => void): () => void {
    const wrapped = handler as Handler;
    const existing = listeners.get(event);

    if (existing) {
      existing.add(wrapped);
    } else {
      listeners.set(event, new Set([wrapped]));
      if (feed) attach(feed, event);
    }
    connect();

    return () => {
      listeners.get(event)?.delete(wrapped);
    };
  },

  async openUrl(url: string): Promise<void> {
    // `mailto:` and `https:` both work here; the browser decides what opening
    // one means.
    window.open(url, "_blank", "noopener,noreferrer");
  },

  async copyToClipboard(text: string): Promise<void> {
    await navigator.clipboard.writeText(text);
  },

  async chooseImportDocument(): Promise<ImportDocumentReader | null> {
    const file = await pickFile();
    return file ? readDocument(file) : null;
  },

  // Watched on the window so the whole page is the drop target, which is what
  // the desktop app does and what people expect from a drop zone.
  watchFileDrops(handlers: DropHandlers): () => void {
    // Counted, because dragging over a child element fires a leave on the
    // parent and the highlight would flicker.
    let depth = 0;

    const enter = (event: DragEvent) => {
      if (!carriesFiles(event)) return;
      depth += 1;
      handlers.onOver();
    };

    const over = (event: DragEvent) => {
      // Without this the browser refuses the drop and opens the file instead.
      if (carriesFiles(event)) event.preventDefault();
    };

    const leave = (event: DragEvent) => {
      if (!carriesFiles(event)) return;
      depth = Math.max(0, depth - 1);
      if (depth === 0) handlers.onLeave();
    };

    const drop = (event: DragEvent) => {
      if (!carriesFiles(event)) return;
      event.preventDefault();
      depth = 0;
      handlers.onLeave();
      const file = event.dataTransfer?.files?.[0] ?? null;
      handlers.onDrop(file ? readDocument(file) : null);
    };

    window.addEventListener("dragenter", enter);
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);

    return () => {
      window.removeEventListener("dragenter", enter);
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  },

  exportLibraryMarkdown(): Promise<string | null> {
    return saveDownload("export_library_markdown");
  },

  exportCollectionMarkdown(collection: Collection): Promise<string | null> {
    return saveDownload("export_collection_markdown", { collectionId: collection.id });
  },

  backupLibrary(): Promise<string | null> {
    return saveDownload("backup_library");
  },
};
