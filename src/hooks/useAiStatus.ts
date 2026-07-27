import { useCallback, useEffect, useState } from "react";

import { api } from "../lib/ipc";
import type { AiStatus } from "../lib/types";

export interface AiAvailability {
  status: AiStatus | null;
  /** True only when AI is on, a key is stored, and the key can be reached. */
  ready: boolean;
  refresh: () => void;
}

/**
 * Answers one question for the shell: may AI-backed entry points be shown?
 * The status is re-read whenever the AI switch changes, so turning AI off in
 * Settings hides Import without a restart.
 */
export function useAiStatus(aiEnabled: boolean): AiAvailability {
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [reloads, setReloads] = useState(0);

  const refresh = useCallback(() => setReloads((count) => count + 1), []);

  useEffect(() => {
    if (!aiEnabled) {
      setStatus(null);
      return;
    }

    let active = true;
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

    return () => {
      active = false;
    };
  }, [aiEnabled, reloads]);

  return {
    status,
    ready: aiEnabled && status?.keyStored === true && status.credentialManagerAvailable,
    refresh,
  };
}
