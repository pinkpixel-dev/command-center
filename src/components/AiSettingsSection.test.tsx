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
      saveAiKey: vi.fn(),
      removeAiKey: vi.fn(),
      testAiConnection: vi.fn(),
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
};

const status: AiStatus = {
  keyStored: false,
  credentialManagerAvailable: true,
  defaultModel: "gpt-5.6-luna",
  effectiveModel: "gpt-5.6-luna",
  models: ["gpt-5.6-luna", "gpt-5.6-terra", "gpt-5-nano"],
};

describe("AiSettingsSection", () => {
  beforeEach(() => {
    vi.mocked(api.getAiStatus).mockReset();
    vi.mocked(api.getAiStatus).mockResolvedValue(status);
    vi.mocked(api.saveAiKey).mockReset();
    vi.mocked(api.removeAiKey).mockReset();
    vi.mocked(api.testAiConnection).mockReset();
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
      await screen.findByText(/Secure storage could not be verified/),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add key" }));
    await user.type(screen.getByLabelText("OpenAI API key"), "sk-test-value");
    await user.click(screen.getByRole("button", { name: "Store key" }));

    expect(api.saveAiKey).toHaveBeenCalledWith("sk-test-value");
    expect(await screen.findByText("Key stored securely")).toBeInTheDocument();
    expect(
      screen.queryByText(/Secure storage could not be verified/),
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
});
