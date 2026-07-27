import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import { backupLibraryDatabase, exportLibraryMarkdown } from "../lib/library-files";
import type { AppSettings } from "../lib/types";
import { SettingsPanel } from "./SettingsPanel";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      libraryLocation: vi.fn().mockResolvedValue("/tmp/command-center/library.db"),
      getAiStatus: vi.fn().mockResolvedValue({
        keyStored: false,
        credentialManagerAvailable: true,
        defaultModel: "gpt-5.6-luna",
        effectiveModel: "gpt-5.6-luna",
        models: ["gpt-5.6-luna", "gpt-5.6-terra"],
      }),
    },
  };
});

vi.mock("../lib/library-files", () => ({
  exportLibraryMarkdown: vi.fn(),
  backupLibraryDatabase: vi.fn(),
}));

const settings: AppSettings = {
  theme: "dark",
  commandViewMode: "compact",
  confirmBeforeDelete: true,
  launchAtStartup: false,
  closeToTray: false,
  aiEnabled: false,
  aiModel: null,
};

describe("SettingsPanel", () => {
  beforeEach(() => {
    vi.mocked(api.libraryLocation).mockResolvedValue("/tmp/command-center/library.db");
    vi.mocked(api.getAiStatus).mockResolvedValue({
      keyStored: false,
      credentialManagerAvailable: true,
      defaultModel: "gpt-5.6-luna",
      effectiveModel: "gpt-5.6-luna",
      models: ["gpt-5.6-luna", "gpt-5.6-terra"],
    });
    vi.mocked(exportLibraryMarkdown).mockReset();
    vi.mocked(backupLibraryDatabase).mockReset();
  });

  it("exports Markdown and creates a restorable database backup", async () => {
    const user = userEvent.setup();
    vi.mocked(exportLibraryMarkdown).mockResolvedValue("/tmp/library.md");
    vi.mocked(backupLibraryDatabase).mockResolvedValue("/tmp/library.db");

    render(<SettingsPanel settings={settings} onSave={vi.fn().mockResolvedValue(settings)} />);

    await user.click(screen.getByRole("button", { name: "Export Markdown" }));
    expect(exportLibraryMarkdown).toHaveBeenCalledOnce();
    expect(await screen.findByRole("status")).toHaveTextContent("Markdown export saved");

    await user.click(screen.getByRole("button", { name: "Back up library" }));
    expect(backupLibraryDatabase).toHaveBeenCalledOnce();
    expect(await screen.findByRole("status")).toHaveTextContent("Library backup saved");
  });

  it("saves the high-contrast card view as part of the complete settings payload", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockImplementation(async (next: AppSettings) => next);

    render(<SettingsPanel settings={settings} onSave={onSave} />);

    await user.selectOptions(screen.getByLabelText("Theme"), "high-contrast");
    await user.selectOptions(screen.getByLabelText("Library view"), "cards");
    await user.click(screen.getByLabelText("Launch Command Center when you sign in"));
    await user.click(screen.getByLabelText("Keep running in the tray when the window closes"));

    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save settings" }));

    expect(onSave).toHaveBeenCalledWith({
      ...settings,
      theme: "high-contrast",
      commandViewMode: "cards",
      launchAtStartup: true,
      closeToTray: true,
    });
    expect(await screen.findByRole("status")).toHaveTextContent("Settings saved");
  });

  it("contains no controls for removed Quick Add or default collection preferences", async () => {
    render(
      <SettingsPanel
        settings={settings}
        onSave={vi.fn().mockResolvedValue(settings)}
      />,
    );

    expect(screen.queryByText("Quick Add")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Global shortcut")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Default collection for new commands")).not.toBeInTheDocument();
    expect(await screen.findByText("/tmp/command-center/library.db")).toBeInTheDocument();
  });

  it("does not save an empty custom model ID", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel settings={settings} onSave={vi.fn().mockResolvedValue(settings)} />);

    await user.click(screen.getByLabelText("Enable AI features"));
    await user.selectOptions(await screen.findByLabelText("OpenAI model"), "__custom__");

    expect(screen.getByText("Enter a model ID or choose a model from the list.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save settings" })).toBeDisabled();
  });
});
