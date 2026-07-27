import { useCallback, useRef, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import type { OutboundPlan } from "../lib/outbound";
import { nextRequestId } from "../lib/request-id";
import type { ErrorAnalysis } from "../lib/types";

export interface ErrorAnalysisState {
  /** The local disclosure, once the paste has been described. */
  plan: OutboundPlan | null;
  /** A disclosure is being prepared. Nothing has left the machine. */
  preparing: boolean;
  /** A request is on the wire and can still be stopped. */
  analyzing: boolean;
  error: string | null;
  cancelled: boolean;
  /** Describes the paste locally. No network request. */
  prepare: (output: string) => void;
  /** Sends the paste. Only valid once a disclosure has been shown. */
  analyze: (output: string) => Promise<ErrorAnalysis | null>;
  cancel: () => void;
  /** Back to the paste box, keeping whatever is typed. */
  discard: () => void;
}

/**
 * Owns the two steps a pasted error takes: describe it locally, then send it.
 *
 * The disclosure is deliberately its own step. Pasted terminal output is the
 * one thing a person is most likely to hand over without reading, so the size,
 * the model, and every likely secret are on screen before anything is sent.
 */
export function useErrorAnalysis(ready: boolean): ErrorAnalysisState {
  const [plan, setPlan] = useState<OutboundPlan | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);

  const pending = useRef<number | null>(null);

  const prepare = useCallback(
    (output: string) => {
      if (!ready || !output.trim()) return;

      setPreparing(true);
      setError(null);
      setCancelled(false);
      api
        .prepareErrorAnalysis(output)
        .then(setPlan)
        .catch((caught) => setError(toAppError(caught).message))
        .finally(() => setPreparing(false));
    },
    [ready],
  );

  const analyze = useCallback(
    async (output: string): Promise<ErrorAnalysis | null> => {
      if (!ready || pending.current !== null) return null;

      const requestId = nextRequestId();
      pending.current = requestId;
      setAnalyzing(true);
      setError(null);
      setCancelled(false);

      try {
        return await api.analyzeTerminalError(requestId, output);
      } catch (caught) {
        const payload = toAppError(caught);
        if (payload.kind === "ai_cancelled") {
          setCancelled(true);
        } else {
          setError(payload.message);
        }
        return null;
      } finally {
        pending.current = null;
        setAnalyzing(false);
      }
    },
    [ready],
  );

  const cancel = useCallback(() => {
    const requestId = pending.current;
    if (requestId === null) return;
    void api.cancelAssistantRequest(requestId).catch(() => undefined);
  }, []);

  const discard = useCallback(() => {
    setPlan(null);
    setError(null);
    setCancelled(false);
  }, []);

  return { plan, preparing, analyzing, error, cancelled, prepare, analyze, cancel, discard };
}
