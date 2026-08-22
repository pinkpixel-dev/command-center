import { useCallback, useEffect, useState } from "react";

import { api } from "../lib/ipc";
import type { AiStatus, AppSettings, CodexStatus } from "../lib/types";

export interface AiAvailability {
  status: AiStatus | null;
  codex: CodexStatus | null;
  /** True only when the selected provider is connected and usable. */
  ready: boolean;
  refresh: () => void;
}

/**
 * Answers one question for the shell: may AI-backed entry points be shown?
 * The status is re-read whenever the AI switch or the selected provider
 * changes, so a change in Settings takes effect without a restart.
 *
 * Rust enforces the same gate. This one decides what the user can see, not
 * what the backend will allow.
 */
export function useAiStatus(settings: AppSettings): AiAvailability {
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [codex, setCodex] = useState<CodexStatus | null>(null);
  const [reloads, setReloads] = useState(0);

  const { aiEnabled, aiProvider, codexModel } = settings;
  const refresh = useCallback(() => setReloads((count) => count + 1), []);

  useEffect(() => {
    if (!aiEnabled) {
      setStatus(null);
      setCodex(null);
      return;
    }

    let active = true;

    if (aiProvider === "openaiApi") {
      setCodex(null);
      api
        .getAiStatus()
        .then((result) => {
          if (active) setStatus(result);
        })
        .catch(() => {
          // A status read that fails is not proof of anything, but it is not
          // permission to show a network-backed action either.
          if (active) setStatus(null);
        });
    } else {
      setStatus(null);
      api
        .getCodexStatus()
        .then((result) => {
          if (active) setCodex(result);
        })
        .catch(() => {
          if (active) setCodex(null);
        });
    }

    return () => {
      active = false;
    };
  }, [aiEnabled, aiProvider, reloads]);

  const ready = aiEnabled && isProviderReady(aiProvider, status, codex, codexModel);

  return { status, codex, ready, refresh };
}

function isProviderReady(
  provider: AppSettings["aiProvider"],
  status: AiStatus | null,
  codex: CodexStatus | null,
  codexModel: string | null,
): boolean {
  if (provider === "openaiApi") {
    return status?.keyStored === true && status.credentialManagerAvailable;
  }

  // Codex needs an installation, a connected ChatGPT account, and a model
  // chosen from the live list. There is no curated default to fall back on.
  return (
    codex?.availability.state === "ready" &&
    codex.account.state === "connected" &&
    codexModel !== null
  );
}
