import { useEffect, useRef, useState } from "react";
import { Eraser, RotateCcw, SendHorizontal, X } from "lucide-react";

import { MAX_MESSAGE_CHARS } from "../../lib/assistant";
import type { AssistantContext, AssistantMessage } from "../../lib/assistant";
import type { CommandProposal } from "../../lib/types";
import { Button } from "../ui/Button";
import { AssistantTranscript } from "./AssistantTranscript";

export interface AssistantPanelProps {
  context: AssistantContext | null;
  /** The model the request will actually use, shown before anything is sent. */
  model: string | null;
  messages: AssistantMessage[];
  sending: boolean;
  error: string | null;
  cancelled: boolean;
  retryable: boolean;
  onSend: (text: string) => void;
  onCancel: () => void;
  onRetry: () => void;
  onClear: () => void;
  onClose: () => void;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

/** Warn before the composer refuses, rather than at the moment it refuses. */
const COUNTER_VISIBLE_FROM = MAX_MESSAGE_CHARS - 300;

/**
 * The assistant surface: a docked panel beside the library on desktop, a
 * full-screen sheet on small screens. It is not a modal, so the library stays
 * readable and operable while a question is in flight.
 */
export function AssistantPanel({
  context,
  model,
  messages,
  sending,
  error,
  cancelled,
  retryable,
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

      <div className="assistant__body" ref={logRef}>
        <AssistantTranscript
          messages={messages}
          context={context}
          sending={sending}
          onCopy={onCopy}
          onReview={onReview}
        />

        {cancelled && (
          <p className="assistant__note assistant__note--stopped" role="status">
            Stopped. Nothing was added to the conversation.
          </p>
        )}

        {error && (
          <p className="assistant__error" role="alert">
            {error}
          </p>
        )}

        {retryable && (
          <div className="assistant__retry">
            <Button variant="secondary" size="sm" onClick={onRetry}>
              <RotateCcw size={14} aria-hidden="true" />
              Send it again
            </Button>
          </div>
        )}
      </div>

      <div className="assistant__foot">
        <p className="assistant__disclosure" id="assistant-disclosure">
          {context ? `Sent with this entry to ${model ?? "OpenAI"}. ` : `Sent to ${model ?? "OpenAI"}. `}
          Likely secrets are replaced first. Answers are AI-generated and may be wrong.
        </p>

        <div className="assistant__composer">
          <textarea
            id="assistant-composer"
            ref={composerRef}
            className="input assistant__input"
            rows={2}
            value={draft}
            placeholder={context ? "Ask about this entry" : "Ask for a command"}
            aria-label="Message the assistant"
            aria-describedby="assistant-disclosure"
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
    </aside>
  );
}
