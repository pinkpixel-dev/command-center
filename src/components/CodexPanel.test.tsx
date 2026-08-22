import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { AppSettings, CodexStatus } from "../lib/types";
import { CodexPanel } from "./CodexPanel";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      getCodexStatus: vi.fn(),
      refreshCodex: vi.fn(),
      listCodexModels: vi.fn(),
    },
  };
});

const settings: AppSettings = {
  theme: "dark",
  commandViewMode: "cards",
  confirmBeforeDelete: true,
  launchAtStartup: false,
  closeToTray: false,
  aiEnabled: true,
  aiModel: null,
  aiProvider: "chatgptCodex",
  codexModel: null,
  codexPath: null,
};

function status(overrides: Partial<CodexStatus> = {}): CodexStatus {
  return {
    availability: { state: "ready", version: "0.147.0" },
    account: { state: "notConnected" },
    accountError: null,
    diagnostics: [],
    ...overrides,
  };
}

function renderPanel(overrides: Partial<AppSettings> = {}, onPatch = vi.fn()) {
  const draft = { ...settings, ...overrides };
  return {
    onPatch,
    ...render(<CodexPanel draft={draft} saved={settings} onPatch={onPatch} />),
  };
}

describe("CodexPanel", () => {
  beforeEach(() => {
    vi.mocked(api.getCodexStatus).mockReset();
    vi.mocked(api.refreshCodex).mockReset();
    vi.mocked(api.getCodexStatus).mockResolvedValue(status());
    vi.mocked(api.listCodexModels).mockReset();
    vi.mocked(api.listCodexModels).mockResolvedValue([]);
  });

  it("reports the installed version when Codex is usable", async () => {
    renderPanel();

    expect(await screen.findByText("Codex 0.147.0 found")).toBeInTheDocument();
  });

  it("explains how to install Codex when none is found", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "notFound" } }),
    );

    renderPanel();

    expect(await screen.findByText("Codex was not found")).toBeInTheDocument();
    // The install command is the whole point of this state: without it the
    // user is told what is wrong and nothing about how to fix it.
    expect(screen.getByText("npm install -g @openai/codex")).toBeInTheDocument();
  });

  it("names both versions when the installed Codex is too old", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({
        availability: { state: "tooOld", version: "0.100.0", minimum: "0.147.0" },
      }),
    );

    renderPanel();

    expect(await screen.findByText("Codex 0.100.0 is too old")).toBeInTheDocument();
    expect(screen.getByText(/needs 0\.147\.0 or newer/)).toBeInTheDocument();
  });

  it("gives a specific reason when the path points at a launcher script", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "unusable", reason: "shimNotSupported" } }),
    );

    renderPanel();

    expect(await screen.findByText(/launcher script/)).toBeInTheDocument();
  });

  it("re-runs discovery when the user asks to check again", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "notFound" } }),
    );
    vi.mocked(api.refreshCodex).mockResolvedValue(status());

    renderPanel();
    await screen.findByText("Codex was not found");

    await user.click(screen.getByRole("button", { name: "Check again" }));

    await waitFor(() => expect(api.refreshCodex).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("Codex 0.147.0 found")).toBeInTheDocument();
  });

  it("keeps the path field out of the way when Codex was found", async () => {
    renderPanel();
    await screen.findByText("Codex 0.147.0 found");

    // Most people never need it, so a working install shows the status and
    // nothing to configure.
    expect(screen.queryByLabelText("Codex program path")).not.toBeInTheDocument();
  });

  it("offers the path field when discovery could not find Codex", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "notFound" } }),
    );

    renderPanel();
    await screen.findByText("Codex was not found");

    expect(screen.getByLabelText("Codex program path")).toBeInTheDocument();
  });

  it("keeps the path field visible when one is already saved", async () => {
    renderPanel({ codexPath: "/opt/codex" });
    await screen.findByText("Codex 0.147.0 found");

    // A saved path has to stay changeable and clearable.
    expect(screen.getByLabelText("Codex program path")).toHaveValue("/opt/codex");
  });

  it("records a manual path without saving it directly", async () => {
    const user = userEvent.setup();
    const onPatch = vi.fn();
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "notFound" } }),
    );

    renderPanel({}, onPatch);
    await screen.findByText("Codex was not found");

    await user.type(screen.getByLabelText("Codex program path"), "/opt/codex");

    // The panel edits the draft. Saving stays with the Settings save action.
    expect(onPatch).toHaveBeenCalled();
    expect(api.refreshCodex).not.toHaveBeenCalled();
  });

  it("tells the user to save before an edited path takes effect", async () => {
    renderPanel({ codexPath: "/opt/codex" });
    await screen.findByText("Codex 0.147.0 found");

    expect(
      await screen.findByText(/Save settings, then choose Check again/),
    ).toBeInTheDocument();
  });

  it("offers the account section once Codex is usable", async () => {
    renderPanel();
    await screen.findByText("Codex 0.147.0 found");

    expect(
      await screen.findByRole("button", { name: "Connect ChatGPT" }),
    ).toBeInTheDocument();
  });

  it("keeps the account section hidden while Codex is unusable", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "notFound" } }),
    );

    renderPanel();
    await screen.findByText("Codex was not found");

    // Offering a sign-in with no Codex to run it would be a dead end.
    expect(
      screen.queryByRole("button", { name: "Connect ChatGPT" }),
    ).not.toBeInTheDocument();
  });

  it("says the platform cannot run Codex without offering a path field", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ availability: { state: "unsupportedPlatform" } }),
    );

    renderPanel();

    expect(
      await screen.findByText("Codex is not available on this platform"),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("Codex program path")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Check again" })).not.toBeInTheDocument();
  });

  it("keeps diagnostics behind a toggle rather than in the main flow", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ diagnostics: ["Declined a tool request."] }),
    );

    renderPanel();
    await screen.findByText("Codex 0.147.0 found");

    expect(screen.queryByText("Declined a tool request.")).not.toBeInTheDocument();

    const toggle = screen.getByRole("button", { name: "Show details" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await user.click(toggle);

    expect(screen.getByText("Declined a tool request.")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Hide details" }),
    ).toHaveAttribute("aria-expanded", "true");
  });

  it("surfaces an account read failure as an alert", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      status({ accountError: "Codex did not answer the account/read request in time." }),
    );

    renderPanel();

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/account\/read/);
  });

  it("shows a failed status read without pretending Codex is ready", async () => {
    vi.mocked(api.getCodexStatus).mockRejectedValue({
      kind: "runtime",
      message: "Codex status is unavailable.",
    });

    renderPanel();

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Codex status is unavailable.",
    );
    expect(screen.queryByText(/found/)).not.toBeInTheDocument();
  });
});
