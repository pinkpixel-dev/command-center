import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  backupLibraryDatabase,
  exportCollectionMarkdown,
  exportLibraryMarkdown,
} from "./library-files";

// These run against the desktop platform, which is what `@platform` resolves
// to under test. Mocking Tauri itself rather than the layer above means the
// save dialog and the command it feeds are both covered.
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const collection = {
  id: 7,
  name: "Docker Tools",
  description: "",
  commandCount: 3,
  createdAt: "2026-07-01T00:00:00Z",
  updatedAt: "2026-07-01T00:00:00Z",
};

describe("library file actions", () => {
  beforeEach(() => {
    vi.mocked(save).mockReset();
    vi.mocked(invoke).mockReset();
  });

  it("exports only the chosen collection with a readable default file name", async () => {
    vi.mocked(save).mockResolvedValue("/tmp/docker-tools.md");
    vi.mocked(invoke).mockResolvedValue("/tmp/docker-tools.md");

    await expect(exportCollectionMarkdown(collection)).resolves.toBe("/tmp/docker-tools.md");

    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Export Docker Tools",
        defaultPath: expect.stringMatching(/^command-center-docker-tools-\d{4}-\d{2}-\d{2}\.md$/),
      }),
    );
    expect(invoke).toHaveBeenCalledWith("export_collection_markdown", {
      collectionId: 7,
      destination: "/tmp/docker-tools.md",
    });
  });

  it("exports Markdown only after a destination is selected", async () => {
    vi.mocked(save).mockResolvedValue("/tmp/command-center.md");
    vi.mocked(invoke).mockResolvedValue("/tmp/command-center.md");

    await expect(exportLibraryMarkdown()).resolves.toBe("/tmp/command-center.md");
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Export Command Center library",
        filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
      }),
    );
    expect(invoke).toHaveBeenCalledWith("export_library_markdown", {
      destination: "/tmp/command-center.md",
    });
  });

  it("backs up SQLite and treats closing the dialog as a clean cancellation", async () => {
    vi.mocked(save).mockResolvedValueOnce("/tmp/command-center.db");
    vi.mocked(invoke).mockResolvedValue("/tmp/command-center.db");

    await expect(backupLibraryDatabase()).resolves.toBe("/tmp/command-center.db");
    expect(invoke).toHaveBeenCalledWith("backup_library", {
      destination: "/tmp/command-center.db",
    });

    vi.mocked(save).mockResolvedValueOnce(null);
    await expect(backupLibraryDatabase()).resolves.toBeNull();
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
