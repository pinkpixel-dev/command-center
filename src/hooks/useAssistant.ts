import { useCallback, useEffect, useRef, useState } from "react";

import { lastQuestion, messageId, toTurns } from "../lib/assistant";
import type { AssistantContext, AssistantMessage } from "../lib/assistant";
import { api, toAppError } from "../lib/ipc";

export interface AssistantState {
  messages: AssistantMessage[];
  /** A request is on the wire and can still be cancelled. */
  sending: boolean;
  error: string | null;
  /** The last request was stopped by the user rather than by a failure. */
  cancelled: boolean;
  /** True when the last exchange failed and can be sent again. */
  retryable: boolean;
  send: (text: string) => void;
  cancel: () => void;
  retry: () => void;
  clear: () => void;
}

/**
 * Owns one conversation, in memory only. Switching entries or turning AI off
 * starts over, because a conversation about one command has nothing useful to
 * say about the next one.
 */
export function useAssistant(context: AssistantContext | null, ready: boolean): AssistantState {
  const [messages, setMessages] = useState<AssistantMessage[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);

  const requestCount = useRef(0);
  const sequence = useRef(0);
  // The request the panel is currently waiting for. Anything else that lands
  // belongs to a conversation the user has already moved on from.
  const pending = useRef<number | null>(null);
  // Send and Retry need the transcript as it stands without depending on it,
  // which would rebuild both callbacks on every message.
  const transcript = useRef<AssistantMessage[]>(messages);
  transcript.current = messages;
  const contextId = context?.commandId ?? null;

  const reset = useCallback(() => {
    if (pending.current !== null) {
      void api.cancelAssistantRequest(pending.current).catch(() => undefined);
      pending.current = null;
    }
    setMessages([]);
    setError(null);
    setCancelled(false);
    setSending(false);
  }, []);

  useEffect(() => {
    reset();
  }, [contextId, ready, reset]);

  const ask = useCallback(
    (message: string, history: AssistantMessage[]) => {
      const requestId = requestCount.current + 1;
      requestCount.current = requestId;
      pending.current = requestId;
      sequence.current += 1;

      const question: AssistantMessage = {
        id: messageId("user", sequence.current),
        role: "user",
        text: message,
        proposals: [],
      };

      setMessages([...history, question]);
      setError(null);
      setCancelled(false);
      setSending(true);

      api
        .askAssistant({
          requestId,
          commandId: contextId,
          turns: toTurns(history),
          message,
        })
        .then((reply) => {
          if (pending.current !== requestId) return;
          sequence.current += 1;
          setMessages((current) => [
            ...current,
            {
              id: messageId("assistant", sequence.current),
              role: "assistant",
              text: reply.reply,
              proposals: reply.proposals,
            },
          ]);
        })
        .catch((caught) => {
          if (pending.current !== requestId) return;
          const payload = toAppError(caught);
          if (payload.kind === "ai_cancelled") {
            setCancelled(true);
          } else {
            setError(payload.message);
          }
          // The question stays on screen so Retry has something to resend, but
          // it is not sent as context for anything else.
          setMessages((current) =>
            current.map((entry) =>
              entry.id === question.id ? { ...entry, failed: true } : entry,
            ),
          );
        })
        .finally(() => {
          if (pending.current !== requestId) return;
          pending.current = null;
          setSending(false);
        });
    },
    [contextId],
  );

  const send = useCallback(
    (text: string) => {
      const trimmed = text.trim();
      if (!ready || !trimmed || pending.current !== null) return;
      ask(trimmed, transcript.current);
    },
    [ask, ready],
  );

  const retry = useCallback(() => {
    if (!ready || pending.current !== null) return;
    const previous = lastQuestion(transcript.current);
    if (previous) ask(previous.message, previous.history);
  }, [ask, ready]);

  const cancel = useCallback(() => {
    const requestId = pending.current;
    if (requestId === null) return;
    void api.cancelAssistantRequest(requestId).catch(() => undefined);
  }, []);

  return {
    messages,
    sending,
    error,
    cancelled,
    retryable: !sending && messages.some((message) => message.failed === true),
    send,
    cancel,
    retry,
    clear: reset,
  };
}
