import { useCallback, useEffect, useRef, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import { nextRequestId } from "../lib/request-id";
import type { ShellConversion, ShellOption, TargetShell } from "../lib/types";

export interface ShellConversionState {
  /** The shells the backend will convert to, so the picker cannot drift. */
  shells: ShellOption[];
  result: ShellConversion | null;
  converting: boolean;
  error: string | null;
  cancelled: boolean;
  convert: (target: TargetShell) => void;
  cancel: () => void;
  /** Clears the result so the picker comes back. */
  reset: () => void;
}

/**
 * Owns one entry's conversion. Nothing is cached: a conversion is a one-off
 * question about a command the user is looking at, and keeping a stale one
 * beside an edited entry would be worse than asking again.
 */
export function useShellConversion(commandId: number, ready: boolean): ShellConversionState {
  const [shells, setShells] = useState<ShellOption[]>([]);
  const [result, setResult] = useState<ShellConversion | null>(null);
  const [converting, setConverting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);

  const pending = useRef<number | null>(null);
  // The dialog reuses one component instance across entries, so a conversion
  // that lands late must not answer for the entry now on screen.
  const openId = useRef(commandId);
  openId.current = commandId;

  useEffect(() => {
    if (!ready || shells.length > 0) return;

    let active = true;
    api
      .conversionShells()
      .then((options) => {
        if (active) setShells(options);
      })
      .catch(() => undefined);

    return () => {
      active = false;
    };
  }, [ready, shells.length]);

  useEffect(() => {
    setResult(null);
    setError(null);
    setCancelled(false);
  }, [commandId]);

  const convert = useCallback(
    (target: TargetShell) => {
      if (!ready || pending.current !== null) return;

      const requestId = nextRequestId();
      pending.current = requestId;
      setConverting(true);
      setError(null);
      setCancelled(false);

      api
        .convertCommandShell(requestId, commandId, target)
        .then((conversion) => {
          if (openId.current === commandId) setResult(conversion);
        })
        .catch((caught) => {
          if (openId.current !== commandId) return;
          const payload = toAppError(caught);
          if (payload.kind === "ai_cancelled") {
            setCancelled(true);
          } else {
            setError(payload.message);
          }
        })
        .finally(() => {
          pending.current = null;
          if (openId.current === commandId) setConverting(false);
        });
    },
    [commandId, ready],
  );

  const cancel = useCallback(() => {
    const requestId = pending.current;
    if (requestId === null) return;
    void api.cancelAssistantRequest(requestId).catch(() => undefined);
  }, []);

  const reset = useCallback(() => {
    setResult(null);
    setError(null);
    setCancelled(false);
  }, []);

  return { shells, result, converting, error, cancelled, convert, cancel, reset };
}
