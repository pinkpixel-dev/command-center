import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { AppSettings } from "../lib/types";
import { SettingsPanel } from "./SettingsPanel";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      libraryLocation: vi.fn().mockResolvedValue("/tmp/command-center/library.db"),
    },
  };
});

const settings: AppSettings = {
  theme: "dark",
  commandViewMode: "compact",
  confirmBeforeDelete: true,
};

describe("SettingsPanel", () => {
  beforeEach(() => {
    vi.mocked(api.libraryLocation).mockResolvedValue("/tmp/command-center/library.db");
  });

  it("saves the high-contrast card view as part of the complete settings payload", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockImplementation(async (next: AppSettings) => next);

    render(<SettingsPanel settings={settings} onSave={onSave} />);

    await user.selectOptions(screen.getByLabelText("Theme"), "high-contrast");
    await user.selectOptions(screen.getByLabelText("Library view"), "cards");

    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save settings" }));

    expect(onSave).toHaveBeenCalledWith({
      ...settings,
      theme: "high-contrast",
      commandViewMode: "cards",
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
});
