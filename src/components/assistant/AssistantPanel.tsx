import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Eraser, RotateCcw, SendHorizontal, X } from "lucide-react";

import { MAX_MESSAGE_CHARS } from "../../lib/assistant";
import type { AssistantContext, AssistantMessage } from "../../lib/assistant";
import type { CommandProposal } from "../../lib/types";
import { Button } from "../ui/Button";
import { AssistantTranscript } from "./AssistantTranscript";

/** Asking a question, or handing over terminal output to be read. */
export type AssistantMode = "chat" | "paste";

export interface AssistantPanelProps {
  context: AssistantContext | null;
  messages: AssistantMessage[];
  sending: boolean;
  error: string | null;
  cancelled: boolean;
  retryable: boolean;
  mode: AssistantMode;
  /** The paste-and-disclose surface, rendered in place of the transcript. */
  pasteView: ReactNode;
  /** False once a conversation exists, so switching cannot discard it. */
  canSwitchMode: boolean;
  onModeChange: (mode: AssistantMode) => void;
  onSend: (text: string) => void;
  onCancel: () => void;
  onRetry: () => void;
  onClear: () => void;
  onClose: () => void;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

const MODE_LABELS: Record<AssistantMode, string> = {
  chat: "Ask",
  paste: "Read an error",
};

/** Warn before the composer refuses, rather than at the moment it refuses. */
const COUNTER_VISIBLE_FROM = MAX_MESSAGE_CHARS - 300;

/**
 * The assistant surface: a docked panel beside the library on desktop, a
 * full-screen sheet on small screens. It is not a modal, so the library stays
 * readable and operable while a question is in flight.
 */
export function AssistantPanel({
  context,
  messages,
  sending,
  error,
  cancelled,
  retryable,
  mode,
  pasteView,
  canSwitchMode,
  onModeChange,
  onSend,
  onCancel,
  onRetry,
  onClear,
  onClose,
  onCopy,
  onReview,
}: AssistantPanelProps) {
  const [draft, setDraft] = useState("");
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    returnFocusRef.current = document.activeElement as HTMLElement | null;
    composerRef.current?.focus();
    return () => returnFocusRef.current?.focus();
  }, []);

  // New answers arrive at the bottom, which is where the reader is looking.
  useEffect(() => {
    const log = logRef.current;
    if (log) log.scrollTop = log.scrollHeight;
  }, [messages, sending]);

  const submit = () => {
    const trimmed = draft.trim();
    if (!trimmed || sending) return;
    onSend(trimmed);
    setDraft("");
  };

  const overLimit = draft.length > MAX_MESSAGE_CHARS;

  return (
    <aside
      className="assistant"
      aria-label="Command assistant"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <header className="assistant__head">
        <div className="assistant__heading">
          <h2 className="assistant__title">Assistant</h2>
          {context && (
            <p className="assistant__subject" title={context.title}>
              {context.title}
            </p>
          )}
        </div>
        <div className="assistant__head-actions">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Clear conversation"
            title="Clear conversation"
            disabled={messages.length === 0 && !error && !cancelled}
            onClick={onClear}
          >
            <Eraser size={16} aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Close the assistant"
            title="Close the assistant"
            onClick={onClose}
          >
            <X size={16} aria-hidden="true" />
          </Button>
        </div>
      </header>

      {canSwitchMode && (
        <div className="assistant__modes" role="group" aria-label="What the assistant should do">
          {(["chat", "paste"] as const).map((option) => (
            <button
              key={option}
              type="button"
              className={`assistant__mode${mode === option ? " is-active" : ""}`}
              aria-pressed={mode === option}
              onClick={() => onModeChange(option)}
            >
              {MODE_LABELS[option]}
            </button>
          ))}
        </div>
      )}

      <div className="assistant__body" ref={logRef}>
        {mode === "paste" ? (
          pasteView
        ) : (
          <AssistantTranscript
            messages={messages}
            context={context}
            sending={sending}
            onCopy={onCopy}
            onReview={onReview}
          />
        )}

        {mode === "chat" && cancelled && (
          <p className="assistant__note assistant__note--stopped" role="status">
            Stopped. Nothing was added to the conversation.
          </p>
        )}

        {mode === "chat" && error && (
          <p className="assistant__error" role="alert">
            {error}
          </p>
        )}

        {mode === "chat" && retryable && (
          <div className="assistant__retry">
            <Button variant="secondary" size="sm" onClick={onRetry}>
              <RotateCcw size={14} aria-hidden="true" />
              Send it again
            </Button>
          </div>
        )}
      </div>

      {mode === "chat" && (
      <div className="assistant__foot">
        <div className="assistant__composer">
          <textarea
            id="assistant-composer"
            ref={composerRef}
            className="input assistant__input"
            rows={2}
            value={draft}
            placeholder={composerPlaceholder(context)}
            aria-label="Message the assistant"
            aria-invalid={overLimit || undefined}
            spellCheck
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                submit();
              }
            }}
          />

          {sending ? (
            <Button variant="secondary" size="sm" onClick={onCancel}>
              <X size={15} aria-hidden="true" />
              Stop
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              disabled={draft.trim().length === 0 || overLimit}
              onClick={submit}
            >
              <SendHorizontal size={15} aria-hidden="true" />
              Send
            </Button>
          )}
        </div>

        {draft.length >= COUNTER_VISIBLE_FROM && (
          <p className={`assistant__counter${overLimit ? " is-over" : ""}`} role="status">
            {draft.length} of {MAX_MESSAGE_CHARS} characters
            {overLimit ? ". Shorten it before sending." : ""}
          </p>
        )}
      </div>
      )}
    </aside>
  );
}

function composerPlaceholder(context: AssistantContext | null): string {
  if (context === null) return "Ask for a command";
  return context.kind === "entry" ? "Ask about this entry" : "Ask about this error";
}
