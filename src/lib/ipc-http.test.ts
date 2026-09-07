import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { filenameFrom, platform } from "./ipc-http";
import type { ImportDocumentReader } from "./platform";

/**
 * The web client, tested directly rather than through `@platform`, which
 * resolves to the desktop implementation under test.
 */

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("the web client", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("posts the argument object the desktop app sends to Tauri", async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse([{ id: 1 }]));

    await expect(platform.call("list_commands", { filter: { favorites: true } })).resolves.toEqual([
      { id: 1 },
    ]);

    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe("/api/list_commands");
    expect(init?.method).toBe("POST");
    expect(init?.body).toBe(JSON.stringify({ filter: { favorites: true } }));
  });

  it("sends an empty object for a command that takes no arguments", async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(null));

    await platform.call("library_stats");

    expect(vi.mocked(fetch).mock.calls[0][1]?.body).toBe("{}");
  });

  it("rejects with the kind and message the app already handles", async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse({ kind: "not_found", message: "command 7 was not found" }, 404),
    );

    await expect(platform.call("get_command", { id: 7 })).rejects.toEqual({
      kind: "not_found",
      message: "command 7 was not found",
    });
  });

  /** A proxy in the way answers with HTML, not the error shape. */
  it("falls back to the status when the body is not one of our errors", async () => {
    vi.mocked(fetch).mockResolvedValue(new Response("<html>502</html>", { status: 502 }));

    await expect(platform.call("library_stats")).rejects.toMatchObject({
      kind: "runtime",
      message: expect.stringContaining("502"),
    });
  });

  it("says the server is unreachable rather than leaking a fetch failure", async () => {
    vi.mocked(fetch).mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(platform.call("library_stats")).rejects.toMatchObject({
      kind: "network",
      message: expect.stringMatching(/could not reach/i),
    });
  });

  it("uploads the chosen file as the multipart part the server reads", async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse({ name: "notes.md", content: "# Notes" }));

    // A drop and the file picker build the reader the same way, so exercising
    // the drop covers both.
    const file = new File(["# Notes"], "notes.md", { type: "text/markdown" });
    const readers: (ImportDocumentReader | null)[] = [];

    const stop = platform.watchFileDrops({
      onOver: () => {},
      onLeave: () => {},
      onDrop: (reader) => readers.push(reader),
    });
    const event = new Event("drop") as DragEvent;
    Object.defineProperty(event, "dataTransfer", {
      value: { types: ["Files"], files: [file] },
    });
    window.dispatchEvent(event);
    stop();

    const [read] = readers;
    expect(read).not.toBeNull();
    await expect(read?.()).resolves.toEqual({ name: "notes.md", content: "# Notes" });

    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe("/api/read_import_document");
    expect(init?.body).toBeInstanceOf(FormData);
    const part = (init?.body as FormData).get("file") as File;
    // jsdom rebuilds the File when FormData is given a filename, so the name
    // and the size are what there is to check.
    expect(part.name).toBe("notes.md");
    expect(part.size).toBe(file.size);
  });

  it("ignores a drag that carries text so pasting into a field still works", () => {
    const onOver = vi.fn();
    const stop = platform.watchFileDrops({ onOver, onLeave: () => {}, onDrop: () => {} });

    const event = new Event("dragenter") as DragEvent;
    Object.defineProperty(event, "dataTransfer", { value: { types: ["text/plain"] } });
    window.dispatchEvent(event);
    stop();

    expect(onOver).not.toHaveBeenCalled();
  });
});

describe("the download filename", () => {
  it("reads the name the server attached", () => {
    expect(filenameFrom('attachment; filename="command-center-docker.md"')).toBe(
      "command-center-docker.md",
    );
  });

  it("reads an unquoted name and has an answer for a missing header", () => {
    expect(filenameFrom("attachment; filename=library.db")).toBe("library.db");
    expect(filenameFrom(null)).toBeNull();
    expect(filenameFrom("attachment")).toBeNull();
  });
});
