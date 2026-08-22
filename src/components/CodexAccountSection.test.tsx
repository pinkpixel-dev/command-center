import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { AppSettings, CodexStatus } from "../lib/types";
import { CodexAccountSection } from "./CodexAccountSection";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      startCodexLogin: vi.fn(),
      awaitCodexLogin: vi.fn(),
      cancelCodexLogin: vi.fn(),
      disconnectCodex: vi.fn(),
      listCodexModels: vi.fn(),
      testAiConnection: vi.fn(),
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

const connectedStatus = status({
  account: { state: "connected", email: "person@example.com", plan: "Plus" },
});

function renderSection(
  current: CodexStatus = status(),
  overrides: Partial<AppSettings> = {},
  savedOverrides: Partial<AppSettings> = {},
) {
  const onPatch = vi.fn();
  const onStatus = vi.fn();
  const draft = { ...settings, ...overrides };
  const saved = { ...settings, ...overrides, ...savedOverrides };
  render(
    <CodexAccountSection
      status={current}
      draft={draft}
      saved={saved}
      onPatch={onPatch}
      onStatus={onStatus}
    />,
  );
  return { onPatch, onStatus };
}

describe("CodexAccountSection", () => {
  beforeEach(() => {
    for (const mock of [
      api.startCodexLogin,
      api.awaitCodexLogin,
      api.cancelCodexLogin,
      api.disconnectCodex,
      api.listCodexModels,
      api.testAiConnection,
    ]) {
      vi.mocked(mock).mockReset();
    }
    vi.mocked(api.listCodexModels).mockResolvedValue([
      { id: "gpt-5.6-luna", displayName: "GPT-5.6 Luna", isDefault: true },
      { id: "gpt-5.6-terra", displayName: "GPT-5.6 Terra", isDefault: false },
    ]);
  });

  it("offers both sign-in routes when nothing is connected", () => {
    renderSection();

    expect(screen.getByText("Not connected")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Connect ChatGPT" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Use a sign-in code" }),
    ).toBeInTheDocument();
  });

  it("waits for the browser sign-in without showing an address to copy", async () => {
    const user = userEvent.setup();
    vi.mocked(api.startCodexLogin).mockResolvedValue({ mode: "browser" });
    vi.mocked(api.awaitCodexLogin).mockResolvedValue(connectedStatus);

    const { onStatus } = renderSection();
    await user.click(screen.getByRole("button", { name: "Connect ChatGPT" }));

    expect(api.startCodexLogin).toHaveBeenCalledWith(false);
    await waitFor(() => expect(onStatus).toHaveBeenCalledWith(connectedStatus));
    // Rust opens the browser. The authorization URL never reaches the panel.
    expect(screen.queryByText(/https:\/\//)).not.toBeInTheDocument();
  });

  it("shows the code and address for a device-code sign-in", async () => {
    const user = userEvent.setup();
    vi.mocked(api.startCodexLogin).mockResolvedValue({
      mode: "deviceCode",
      verificationUrl: "https://auth.openai.com/device",
      userCode: "ABCD-EFGH",
    });
    vi.mocked(api.awaitCodexLogin).mockImplementation(() => new Promise(() => {}));

    renderSection();
    await user.click(screen.getByRole("button", { name: "Use a sign-in code" }));

    expect(api.startCodexLogin).toHaveBeenCalledWith(true);
    expect(await screen.findByText("ABCD-EFGH")).toBeInTheDocument();
    expect(screen.getByText("https://auth.openai.com/device")).toBeInTheDocument();
  });

  it("can cancel a sign-in that is still waiting", async () => {
    const user = userEvent.setup();
    vi.mocked(api.startCodexLogin).mockResolvedValue({ mode: "browser" });
    vi.mocked(api.awaitCodexLogin).mockImplementation(() => new Promise(() => {}));
    vi.mocked(api.cancelCodexLogin).mockResolvedValue(undefined);

    renderSection();
    await user.click(screen.getByRole("button", { name: "Connect ChatGPT" }));

    const cancel = await screen.findByRole("button", { name: "Cancel sign-in" });
    await user.click(cancel);

    await waitFor(() => expect(api.cancelCodexLogin).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("Not connected")).toBeInTheDocument();
  });

  it("reports a failed sign-in instead of appearing connected", async () => {
    const user = userEvent.setup();
    vi.mocked(api.startCodexLogin).mockResolvedValue({ mode: "browser" });
    vi.mocked(api.awaitCodexLogin).mockRejectedValue({
      kind: "ai_auth",
      message: "ChatGPT sign-in did not finish.",
    });

    const { onStatus } = renderSection();
    await user.click(screen.getByRole("button", { name: "Connect ChatGPT" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "ChatGPT sign-in did not finish.",
    );
    expect(onStatus).not.toHaveBeenCalled();
    expect(screen.getByText("Not connected")).toBeInTheDocument();
  });

  it("shows the connected account and its plan", async () => {
    renderSection(connectedStatus);

    expect(screen.getByText("person@example.com · Plus")).toBeInTheDocument();
    await waitFor(() => expect(api.listCodexModels).toHaveBeenCalled());
  });

  it("offers the live model list rather than an invented default", async () => {
    renderSection(connectedStatus);

    expect(
      await screen.findByRole("option", { name: "GPT-5.6 Luna (default)" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "GPT-5.6 Terra" })).toBeInTheDocument();
    // Nothing is chosen for the user.
    expect(screen.getByLabelText("Codex model")).toHaveValue("");
  });

  it("says so when the model list cannot be read, without guessing one", async () => {
    vi.mocked(api.listCodexModels).mockRejectedValue({
      kind: "ai_network",
      message: "Codex could not list models.",
    });

    renderSection(connectedStatus);

    expect(await screen.findByText("Codex could not list models.")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /GPT/ })).not.toBeInTheDocument();
  });

  it("asks for a new choice when a saved model has disappeared", async () => {
    renderSection(connectedStatus, { codexModel: "gpt-5.6-retired" });

    expect(
      await screen.findByText(/no longer available on this account/),
    ).toBeInTheDocument();
  });

  it("confirms before disconnecting and says the CLI login is separate", async () => {
    const user = userEvent.setup();
    renderSection(connectedStatus);

    await user.click(screen.getByRole("button", { name: "Disconnect" }));

    const confirm = screen.getByRole("group", { name: "Disconnect ChatGPT" });
    expect(confirm).toHaveTextContent(/Codex CLI sign-in is separate/);
    expect(api.disconnectCodex).not.toHaveBeenCalled();
  });

  it("clears the saved model when the account is disconnected", async () => {
    const user = userEvent.setup();
    vi.mocked(api.disconnectCodex).mockResolvedValue(status());

    const { onPatch, onStatus } = renderSection(connectedStatus, {
      codexModel: "gpt-5.6-luna",
    });

    await user.click(screen.getByRole("button", { name: "Disconnect" }));
    const confirm = screen.getByRole("group", { name: "Disconnect ChatGPT" });
    await user.click(within(confirm).getByRole("button", { name: "Disconnect" }));

    await waitFor(() => expect(api.disconnectCodex).toHaveBeenCalledTimes(1));
    // The model belonged to the account that just went away.
    expect(onPatch).toHaveBeenCalledWith({ codexModel: null });
    expect(onStatus).toHaveBeenCalled();
  });

  it("will not test until the provider and model are saved", async () => {
    renderSection(connectedStatus, { codexModel: "gpt-5.6-luna" }, { codexModel: null });

    expect(await screen.findByRole("button", { name: "Test connection" })).toBeDisabled();
  });

  it("tests through Codex once everything is saved", async () => {
    const user = userEvent.setup();
    vi.mocked(api.testAiConnection).mockResolvedValue({ model: "gpt-5.6-luna" });

    renderSection(connectedStatus, { codexModel: "gpt-5.6-luna" });

    const test = await screen.findByRole("button", { name: "Test connection" });
    expect(test).toBeEnabled();
    await user.click(test);

    expect(await screen.findByText("Connected with gpt-5.6-luna")).toBeInTheDocument();
  });

  it("says which actions Codex currently covers", async () => {
    renderSection(connectedStatus, { codexModel: "gpt-5.6-luna" });

    // Being told the other actions still use the API key beats watching them
    // silently stay hidden.
    expect(
      await screen.findByText(/other AI actions still use the OpenAI API key/),
    ).toBeInTheDocument();
  });

  it("reports a usage limit as its own message", async () => {
    const user = userEvent.setup();
    vi.mocked(api.testAiConnection).mockRejectedValue({
      kind: "ai_rate_limit",
      message: "Your ChatGPT plan's usage limit was reached.",
    });

    renderSection(connectedStatus, { codexModel: "gpt-5.6-luna" });

    await user.click(await screen.findByRole("button", { name: "Test connection" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("usage limit was reached");
  });
});
