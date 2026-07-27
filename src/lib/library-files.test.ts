import { save } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "./ipc";
import {
  backupLibraryDatabase,
  exportCollectionMarkdown,
  exportLibraryMarkdown,
} from "./library-files";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
}));

vi.mock("./ipc", () => ({
  api: {
    exportLibraryMarkdown: vi.fn(),
    exportCollectionMarkdown: vi.fn(),
    backupLibrary: vi.fn(),
  },
}));

describe("library file actions", () => {
  beforeEach(() => {
    vi.mocked(save).mockReset();
    vi.mocked(api.exportLibraryMarkdown).mockReset();
    vi.mocked(api.exportCollectionMarkdown).mockReset();
    vi.mocked(api.backupLibrary).mockReset();
  });

  it("exports only the chosen collection with a readable default file name", async () => {
    vi.mocked(save).mockResolvedValue("/tmp/docker-tools.md");
    vi.mocked(api.exportCollectionMarkdown).mockResolvedValue("/tmp/docker-tools.md");

    await expect(
      exportCollectionMarkdown({
        id: 7,
        name: "Docker Tools",
        description: "",
        commandCount: 3,
        createdAt: "2026-07-01T00:00:00Z",
        updatedAt: "2026-07-01T00:00:00Z",
      }),
    ).resolves.toBe("/tmp/docker-tools.md");

    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Export Docker Tools",
        defaultPath: expect.stringMatching(/^command-center-docker-tools-\d{4}-\d{2}-\d{2}\.md$/),
      }),
    );
    expect(api.exportCollectionMarkdown).toHaveBeenCalledWith(7, "/tmp/docker-tools.md");
  });

  it("exports Markdown only after a destination is selected", async () => {
    vi.mocked(save).mockResolvedValue("/tmp/command-center.md");
    vi.mocked(api.exportLibraryMarkdown).mockResolvedValue("/tmp/command-center.md");

    await expect(exportLibraryMarkdown()).resolves.toBe("/tmp/command-center.md");
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Export Command Center library",
        filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
      }),
    );
    expect(api.exportLibraryMarkdown).toHaveBeenCalledWith("/tmp/command-center.md");
  });

  it("backs up SQLite and treats closing the dialog as a clean cancellation", async () => {
    vi.mocked(save).mockResolvedValueOnce("/tmp/command-center.db");
    vi.mocked(api.backupLibrary).mockResolvedValue("/tmp/command-center.db");

    await expect(backupLibraryDatabase()).resolves.toBe("/tmp/command-center.db");
    expect(api.backupLibrary).toHaveBeenCalledWith("/tmp/command-center.db");

    vi.mocked(save).mockResolvedValueOnce(null);
    await expect(backupLibraryDatabase()).resolves.toBeNull();
    expect(api.backupLibrary).toHaveBeenCalledTimes(1);
  });
});
