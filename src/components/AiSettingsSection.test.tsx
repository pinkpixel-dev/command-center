import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { AiStatus, AppSettings } from "../lib/types";
import { AiSettingsSection } from "./AiSettingsSection";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      getAiStatus: vi.fn(),
      getCodexStatus: vi.fn(),
      refreshCodex: vi.fn(),
      saveAiKey: vi.fn(),
      removeAiKey: vi.fn(),
      testAiConnection: vi.fn(),
      clearAiExplanations: vi.fn(),
    },
  };
});

const baseSettings: AppSettings = {
  theme: "dark",
  commandViewMode: "cards",
  confirmBeforeDelete: true,
  launchAtStartup: false,
  closeToTray: false,
  aiEnabled: false,
  aiModel: null,
  aiProvider: "openaiApi",
  codexModel: null,
  codexPath: null,
};

const status: AiStatus = {
  keyStored: false,
  credentialManagerAvailable: true,
  defaultModel: "gpt-5.6-luna",
  effectiveModel: "gpt-5.6-luna",
  models: ["gpt-5.6-luna", "gpt-5.6-sol", "gpt-5.6-terra"],
};

describe("AiSettingsSection", () => {
  beforeEach(() => {
    vi.mocked(api.getAiStatus).mockReset();
    vi.mocked(api.getAiStatus).mockResolvedValue(status);
    vi.mocked(api.saveAiKey).mockReset();
    vi.mocked(api.removeAiKey).mockReset();
    vi.mocked(api.testAiConnection).mockReset();
    vi.mocked(api.clearAiExplanations).mockReset();
    vi.mocked(api.getCodexStatus).mockReset();
    vi.mocked(api.getCodexStatus).mockResolvedValue({
      availability: { state: "ready", version: "0.147.0" },
      account: { state: "notConnected" },
      accountError: null,
      diagnostics: [],
    });
    vi.mocked(api.refreshCodex).mockReset();
  });

  it("defaults an existing installation to the OpenAI API key provider", async () => {
    const enabled = { ...baseSettings, aiEnabled: true };

    render(<AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />);

    expect(await screen.findByLabelText("AI provider")).toHaveValue("openaiApi");
    expect(screen.getByLabelText("OpenAI model")).toBeInTheDocument();
    await waitFor(() => expect(api.getCodexStatus).not.toHaveBeenCalled());
  });

  it("switches panels without touching either model selection", async () => {
    const user = userEvent.setup();
    const onPatch = vi.fn();
    const enabled = {
      ...baseSettings,
      aiEnabled: true,
      aiModel: "gpt-5.6-terra",
      codexModel: "gpt-5.6-luna",
    };

    render(<AiSettingsSection draft={enabled} saved={enabled} onPatch={onPatch} />);
    await screen.findByLabelText("AI provider");

    await user.selectOptions(screen.getByLabelText("AI provider"), "chatgptCodex");

    // Only the provider changes. Overwriting the other provider's model here
    // would silently lose a saved choice.
    expect(onPatch).toHaveBeenLastCalledWith({ aiProvider: "chatgptCodex" });
  });

  it("shows the Codex panel and hides the API key controls for the ChatGPT provider", async () => {
    const codexSelected = {
      ...baseSettings,
      aiEnabled: true,
      aiProvider: "chatgptCodex" as const,
    };

    render(
      <AiSettingsSection draft={codexSelected} saved={codexSelected} onPatch={vi.fn()} />,
    );

    expect(await screen.findByText("Codex 0.147.0 found")).toBeInTheDocument();
    expect(screen.queryByLabelText("OpenAI model")).not.toBeInTheDocument();
    // The key controls, not the provider option that shares their name.
    expect(screen.queryByRole("button", { name: "Add key" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Test connection" }),
    ).not.toBeInTheDocument();
  });

  it("keeps AI configuration hidden while the opt-in switch is off", async () => {
    render(
      <AiSettingsSection
        draft={baseSettings}
        saved={baseSettings}
        onPatch={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("Enable AI features")).not.toBeChecked();
    expect(screen.queryByLabelText("OpenAI model")).not.toBeInTheDocument();
    await waitFor(() => expect(api.getAiStatus).not.toHaveBeenCalled());
  });

  it("offers the Rust-provided list and an unrestricted custom model path", async () => {
    const user = userEvent.setup();
    const onPatch = vi.fn();
    const enabled = { ...baseSettings, aiEnabled: true };

    const { rerender } = render(
      <AiSettingsSection draft={enabled} saved={baseSettings} onPatch={onPatch} />,
    );

    await screen.findByRole("option", { name: "gpt-5.6-terra" });
    await user.selectOptions(screen.getByLabelText("OpenAI model"), "__custom__");
    expect(onPatch).toHaveBeenLastCalledWith({ aiModel: "" });

    rerender(
      <AiSettingsSection
        draft={{ ...enabled, aiModel: "ft:gpt-5:team:commands" }}
        saved={baseSettings}
        onPatch={onPatch}
      />,
    );
    expect(screen.getByLabelText("Custom model ID")).toHaveValue(
      "ft:gpt-5:team:commands",
    );
  });

  it("stores a newly entered key and clears the frontend field", async () => {
    const user = userEvent.setup();
    vi.mocked(api.saveAiKey).mockResolvedValue({ keyStored: true });
    const enabled = { ...baseSettings, aiEnabled: true };

    render(
      <AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />,
    );

    await user.click(await screen.findByRole("button", { name: "Add key" }));
    const input = screen.getByLabelText("OpenAI API key");
    await user.type(input, "sk-test-value");
    await user.click(screen.getByRole("button", { name: "Store key" }));

    expect(api.saveAiKey).toHaveBeenCalledWith("sk-test-value");
    expect(await screen.findByText("OpenAI API key stored securely")).toBeInTheDocument();
    expect(screen.queryByDisplayValue("sk-test-value")).not.toBeInTheDocument();
    expect(screen.getByText("Key stored securely")).toBeInTheDocument();
  });

  it("retries secure storage when the preliminary availability check fails", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getAiStatus).mockResolvedValue({
      ...status,
      credentialManagerAvailable: false,
    });
    vi.mocked(api.saveAiKey).mockResolvedValue({ keyStored: true });
    const enabled = { ...baseSettings, aiEnabled: true };

    render(
      <AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />,
    );

    expect(
      await screen.findByText(/Secure storage not verified/),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add key" }));
    await user.type(screen.getByLabelText("OpenAI API key"), "sk-test-value");
    await user.click(screen.getByRole("button", { name: "Store key" }));

    expect(api.saveAiKey).toHaveBeenCalledWith("sk-test-value");
    expect(await screen.findByText("Key stored securely")).toBeInTheDocument();
    expect(
      screen.queryByText(/Secure storage not verified/),
    ).not.toBeInTheDocument();
  });

  it("requires an explicit second action before removing the stored key", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getAiStatus).mockResolvedValue({ ...status, keyStored: true });
    vi.mocked(api.removeAiKey).mockResolvedValue({ keyStored: false });
    const enabled = { ...baseSettings, aiEnabled: true };

    render(
      <AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />,
    );

    await user.click(await screen.findByRole("button", { name: "Remove key" }));
    expect(api.removeAiKey).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Remove stored key" }));

    expect(api.removeAiKey).toHaveBeenCalledOnce();
    expect(await screen.findByText("Stored OpenAI API key removed")).toBeInTheDocument();
  });

  it("tests only a saved enabled configuration with a stored key", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getAiStatus).mockResolvedValue({ ...status, keyStored: true });
    vi.mocked(api.testAiConnection).mockResolvedValue({ model: "gpt-5.6-luna" });
    const enabled = { ...baseSettings, aiEnabled: true };

    render(
      <AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />,
    );

    const button = await screen.findByRole("button", { name: "Test connection" });
    expect(button).toBeEnabled();
    await user.click(button);

    expect(api.testAiConnection).toHaveBeenCalledOnce();
    expect(
      await screen.findByText(
        "Connected with gpt-5.6-luna using Responses and structured output",
      ),
    ).toBeInTheDocument();
  });

  it("removes saved explanations even after AI has been switched off", async () => {
    const user = userEvent.setup();
    vi.mocked(api.clearAiExplanations).mockResolvedValue(3);

    render(
      <AiSettingsSection draft={baseSettings} saved={baseSettings} onPatch={vi.fn()} />,
    );

    await user.click(screen.getByRole("button", { name: "Clear saved explanations" }));

    expect(api.clearAiExplanations).toHaveBeenCalledOnce();
    expect(await screen.findByText("Removed 3 saved explanations")).toBeInTheDocument();
  });

  it("says plainly when there was nothing cached to remove", async () => {
    const user = userEvent.setup();
    vi.mocked(api.clearAiExplanations).mockResolvedValue(0);
    const enabled = { ...baseSettings, aiEnabled: true };

    render(<AiSettingsSection draft={enabled} saved={enabled} onPatch={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Clear saved explanations" }));

    expect(
      await screen.findByText("There were no saved explanations to remove"),
    ).toBeInTheDocument();
  });
});
