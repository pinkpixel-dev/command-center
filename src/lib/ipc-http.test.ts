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

describe("the web client's session", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  const auth = platform.auth!;

  it("reports whether this browser is already signed in", async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse({ authenticated: true }));

    await expect(auth.status()).resolves.toBe(true);
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe("/api/session");
  });

  /** The password goes out once. What comes back is a cookie, not a token
   * this code has to hold on to. */
  it("posts the password and keeps nothing", async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse({ authenticated: true }));

    await auth.signIn("a good long password");

    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe("/api/login");
    expect(init?.body).toBe(JSON.stringify({ password: "a good long password" }));
  });

  it("passes a refused password through as the server worded it", async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse({ kind: "unauthorized", message: "That password was not right." }, 401),
    );

    await expect(auth.signIn("wrong")).rejects.toEqual({
      kind: "unauthorized",
      message: "That password was not right.",
    });
  });

  it("tells the app when any request finds the session gone", async () => {
    const signedOut = vi.fn();
    const stop = auth.onSignedOut(signedOut);
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse({ kind: "unauthorized", message: "Sign in" }, 401),
    );

    await expect(platform.call("library_stats")).rejects.toMatchObject({
      kind: "unauthorized",
    });
    stop();

    expect(signedOut).toHaveBeenCalledTimes(1);
  });

  it("stops reporting to a listener that has gone away", async () => {
    const signedOut = vi.fn();
    auth.onSignedOut(signedOut)();
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse({ kind: "unauthorized", message: "Sign in" }, 401),
    );

    await expect(platform.call("library_stats")).rejects.toMatchObject({ kind: "unauthorized" });

    expect(signedOut).not.toHaveBeenCalled();
  });

  /** Somebody who asked to sign out has to end up signed out even if the
   * request telling the server never arrives. */
  it("signs out locally when the server cannot be reached", async () => {
    const signedOut = vi.fn();
    const stop = auth.onSignedOut(signedOut);
    vi.mocked(fetch).mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(auth.signOut()).rejects.toMatchObject({ kind: "network" });
    stop();

    expect(signedOut).toHaveBeenCalledTimes(1);
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
