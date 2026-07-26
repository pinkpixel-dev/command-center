import { save } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "./ipc";
import { backupLibraryDatabase, exportLibraryMarkdown } from "./library-files";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
}));

vi.mock("./ipc", () => ({
  api: {
    exportLibraryMarkdown: vi.fn(),
    backupLibrary: vi.fn(),
  },
}));

describe("library file actions", () => {
  beforeEach(() => {
    vi.mocked(save).mockReset();
    vi.mocked(api.exportLibraryMarkdown).mockReset();
    vi.mocked(api.backupLibrary).mockReset();
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
