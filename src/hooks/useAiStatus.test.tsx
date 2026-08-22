import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { AiStatus, AppSettings, CodexStatus } from "../lib/types";
import { useAiStatus } from "./useAiStatus";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      getAiStatus: vi.fn(),
      getCodexStatus: vi.fn(),
    },
  };
});

const base: AppSettings = {
  theme: "dark",
  commandViewMode: "cards",
  confirmBeforeDelete: true,
  launchAtStartup: false,
  closeToTray: false,
  aiEnabled: true,
  aiModel: null,
  aiProvider: "openaiApi",
  codexModel: null,
  codexPath: null,
};

const keyReady: AiStatus = {
  keyStored: true,
  credentialManagerAvailable: true,
  defaultModel: "gpt-5.6-luna",
  effectiveModel: "gpt-5.6-luna",
  models: ["gpt-5.6-luna"],
};

function codexStatus(overrides: Partial<CodexStatus> = {}): CodexStatus {
  return {
    availability: { state: "ready", version: "0.147.0" },
    account: { state: "connected", email: "person@example.com", plan: "plus" },
    accountError: null,
    diagnostics: [],
    ...overrides,
  };
}

describe("useAiStatus", () => {
  beforeEach(() => {
    vi.mocked(api.getAiStatus).mockReset();
    vi.mocked(api.getCodexStatus).mockReset();
    vi.mocked(api.getAiStatus).mockResolvedValue(keyReady);
    vi.mocked(api.getCodexStatus).mockResolvedValue(codexStatus());
  });

  it("asks for nothing while AI is off", async () => {
    const { result } = renderHook(() => useAiStatus({ ...base, aiEnabled: false }));

    expect(result.current.ready).toBe(false);
    await waitFor(() => expect(api.getAiStatus).not.toHaveBeenCalled());
    expect(api.getCodexStatus).not.toHaveBeenCalled();
  });

  it("is ready for the API key provider once a key is stored", async () => {
    const { result } = renderHook(() => useAiStatus(base));

    await waitFor(() => expect(result.current.ready).toBe(true));
    // The Codex status is irrelevant to this provider and is not requested.
    expect(api.getCodexStatus).not.toHaveBeenCalled();
  });

  it("is not ready when the credential manager cannot be reached", async () => {
    vi.mocked(api.getAiStatus).mockResolvedValue({
      ...keyReady,
      credentialManagerAvailable: false,
    });

    const { result } = renderHook(() => useAiStatus(base));

    await waitFor(() => expect(api.getAiStatus).toHaveBeenCalled());
    expect(result.current.ready).toBe(false);
  });

  it("reads Codex status but withholds readiness until the workflows are routed", async () => {
    const { result } = renderHook(() =>
      useAiStatus({ ...base, aiProvider: "chatgptCodex", codexModel: "gpt-5.6-luna" }),
    );

    await waitFor(() => expect(result.current.codex).not.toBeNull());
    // A fully connected account is still not enough: the six workflows send
    // their requests through the API key path, so showing their entry points
    // would offer actions that cannot run.
    expect(result.current.ready).toBe(false);
    expect(api.getAiStatus).not.toHaveBeenCalled();
  });

  it("is not ready for Codex without a chosen model", async () => {
    const { result } = renderHook(() =>
      useAiStatus({ ...base, aiProvider: "chatgptCodex", codexModel: null }),
    );

    await waitFor(() => expect(api.getCodexStatus).toHaveBeenCalled());
    expect(result.current.ready).toBe(false);
  });

  it("is not ready for Codex without a connected account", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      codexStatus({ account: { state: "notConnected" } }),
    );

    const { result } = renderHook(() =>
      useAiStatus({ ...base, aiProvider: "chatgptCodex", codexModel: "gpt-5.6-luna" }),
    );

    await waitFor(() => expect(api.getCodexStatus).toHaveBeenCalled());
    expect(result.current.ready).toBe(false);
  });

  it("is not ready for Codex when Codex is missing", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      codexStatus({ availability: { state: "notFound" } }),
    );

    const { result } = renderHook(() =>
      useAiStatus({ ...base, aiProvider: "chatgptCodex", codexModel: "gpt-5.6-luna" }),
    );

    await waitFor(() => expect(api.getCodexStatus).toHaveBeenCalled());
    expect(result.current.ready).toBe(false);
  });

  it("does not let a stored API key make the Codex provider ready", async () => {
    vi.mocked(api.getCodexStatus).mockResolvedValue(
      codexStatus({ account: { state: "notConnected" } }),
    );

    const { result } = renderHook(() =>
      useAiStatus({ ...base, aiProvider: "chatgptCodex", codexModel: "gpt-5.6-luna" }),
    );

    await waitFor(() => expect(api.getCodexStatus).toHaveBeenCalled());
    // The providers are never interchangeable, whatever the other one holds.
    expect(result.current.ready).toBe(false);
    expect(result.current.status).toBeNull();
  });

  it("re-reads status when the provider changes", async () => {
    const { result, rerender } = renderHook((settings: AppSettings) => useAiStatus(settings), {
      initialProps: base,
    });

    await waitFor(() => expect(result.current.ready).toBe(true));

    rerender({ ...base, aiProvider: "chatgptCodex", codexModel: "gpt-5.6-luna" });

    await waitFor(() => expect(api.getCodexStatus).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.status).toBeNull());
  });

  it("treats a failed status read as not ready", async () => {
    vi.mocked(api.getAiStatus).mockRejectedValue(new Error("unreachable"));

    const { result } = renderHook(() => useAiStatus(base));

    await waitFor(() => expect(api.getAiStatus).toHaveBeenCalled());
    expect(result.current.ready).toBe(false);
  });
});
