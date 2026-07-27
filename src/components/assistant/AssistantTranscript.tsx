import { MessageSquareText } from "lucide-react";

import type { AssistantContext, AssistantMessage } from "../../lib/assistant";
import type { CommandProposal } from "../../lib/types";
import { CommandProposalCard } from "./CommandProposalCard";
import { ErrorAnalysisCard } from "./ErrorAnalysisCard";

export interface AssistantTranscriptProps {
  messages: AssistantMessage[];
  context: AssistantContext | null;
  sending: boolean;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

/**
 * The conversation itself. It is a log rather than a dialog, so an answer that
 * arrives while the user is reading is announced without stealing focus.
 */
export function AssistantTranscript({
  messages,
  context,
  sending,
  onCopy,
  onReview,
}: AssistantTranscriptProps) {
  if (messages.length === 0 && !sending) {
    return (
      <div className="assistant__empty">
        <MessageSquareText size={20} aria-hidden="true" />
        <p className="assistant__empty-title">
          {context ? `Ask about ${context.title}` : "Ask for a command"}
        </p>
        <p className="assistant__note">
          {context
            ? "Follow-up questions about this entry, safer alternatives, or what a flag does."
            : "Describe what you are trying to do. Suggested commands arrive with a risk check and never run on their own."}
        </p>
      </div>
    );
  }

  return (
    <div className="assistant__log" role="log" aria-live="polite" aria-label="Conversation">
      {messages.map((message) => (
        <div
          key={message.id}
          className={`assistant__message assistant__message--${message.role}${
            message.failed ? " is-failed" : ""
          }`}
        >
          <p className="assistant__role">{message.role === "user" ? "You" : "Assistant"}</p>
          <p className="assistant__text">{message.text}</p>

          {message.analysis && <ErrorAnalysisCard analysis={message.analysis} />}

          {message.proposals.length > 0 && (
            <div className="assistant__proposals">
              {message.proposals.map((proposal) => (
                <CommandProposalCard
                  key={proposal.command}
                  proposal={proposal}
                  onCopy={onCopy}
                  onReview={onReview}
                />
              ))}
            </div>
          )}
        </div>
      ))}

      {sending && (
        <p className="assistant__pending">
          <span className="assistant__dots" aria-hidden="true" />
          Thinking. This can take a moment.
        </p>
      )}
    </div>
  );
}
