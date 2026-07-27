import { useCallback, useEffect, useRef, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import type { ExplanationView } from "../lib/types";

export interface ExplanationState {
  view: ExplanationView | null;
  /** The cache read that runs when the entry is opened. */
  loading: boolean;
  /** A live request to OpenAI. */
  generating: boolean;
  error: string | null;
  explain: () => void;
  dismissError: () => void;
}

/**
 * Owns one entry's explanation: the cached read on open, and the request the
 * user asks for. Nothing here talks to OpenAI on its own, and the cache read
 * is skipped entirely while AI is unavailable.
 */
export function useExplanation(commandId: number, ready: boolean): ExplanationState {
  const [view, setView] = useState<ExplanationView | null>(null);
  const [loading, setLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The dialog reuses one component instance across entries, so a request that
  // lands after the user moved on must not answer for the entry now on screen.
  const openId = useRef(commandId);
  openId.current = commandId;

  useEffect(() => {
    setView(null);
    setError(null);
    setGenerating(false);
    if (!ready) return;

    let active = true;
    setLoading(true);
    api
      .getCommandExplanation(commandId)
      .then((result) => {
        if (active) setView(result);
      })
      .catch((caught) => {
        if (active) setError(toAppError(caught).message);
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [commandId, ready]);

  const explain = useCallback(() => {
    if (!ready) return;

    setGenerating(true);
    setError(null);
    api
      .explainCommand(commandId)
      .then((result) => {
        if (openId.current === commandId) setView(result);
      })
      .catch((caught) => {
        if (openId.current === commandId) setError(toAppError(caught).message);
      })
      .finally(() => {
        if (openId.current === commandId) setGenerating(false);
      });
  }, [commandId, ready]);

  return {
    view,
    loading,
    generating,
    error,
    explain,
    dismissError: useCallback(() => setError(null), []),
  };
}
