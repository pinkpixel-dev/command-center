/**
 * The assistant conversation as the panel holds it: in memory, for this window
 * session only. Nothing here writes to the database, and closing the panel or
 * turning AI off is the whole of the delete story.
 */

import { emptyCommandInput } from "./types";
import type {
  AssistantTurn,
  CommandInput,
  CommandProposal,
  ErrorAnalysis,
} from "./types";

/** Matches `MAX_MESSAGE_CHARS` in `src-tauri/src/ai/assistant.rs`. */
export const MAX_MESSAGE_CHARS = 2000;

/** Matches `MAX_TURNS` in `src-tauri/src/ai/assistant.rs`. */
export const MAX_TURNS = 12;

export type AssistantRole = "user" | "assistant";

export interface AssistantMessage {
  id: string;
  role: AssistantRole;
  text: string;
  proposals: CommandProposal[];
  /**
   * The structured reading of a pasted error. Only the message that opens an
   * error conversation carries one; follow-ups are ordinary answers.
   */
  analysis?: ErrorAnalysis;
  /** Set when this message is the failed half of an exchange. */
  failed?: boolean;
}

/**
 * What the conversation is about. `null` is general command help.
 *
 * A thread about a saved entry has nothing useful to say about a pasted stack
 * trace, so switching subject starts a new conversation rather than carrying
 * one across.
 */
export type AssistantContext =
  | { kind: "entry"; commandId: number; title: string }
  | {
      kind: "error";
      /** Distinguishes one paste from the next, so a new analysis starts over. */
      sessionId: number;
      /** The paste itself, resent with each follow-up and never stored. */
      output: string;
      analysis: ErrorAnalysis;
      title: string;
    };

/** Stable key for "is this still the same conversation?". */
export function contextKey(context: AssistantContext | null): string {
  if (context === null) return "general";
  return context.kind === "entry"
    ? `entry-${context.commandId}`
    : `error-${context.sessionId}`;
}

/**
 * The first message of an error conversation. The analysis renders as a card,
 * and the text is what a follow-up resends as context, so it has to say the
 * same thing in words.
 */
export function analysisMessage(analysis: ErrorAnalysis): AssistantMessage {
  const text = [analysis.summary, analysis.cause].filter((part) => part.trim()).join(" ");

  return {
    id: messageId("assistant", 0),
    role: "assistant",
    text,
    proposals: analysis.proposals,
    analysis,
  };
}

/**
 * The panel keeps more history than it sends. Rust trims to its own bounds as
 * well, so this is about payload size rather than about being the rule.
 */
export function toTurns(messages: AssistantMessage[]): AssistantTurn[] {
  return messages
    .filter((message) => !message.failed && message.text.trim().length > 0)
    .slice(-MAX_TURNS)
    .map((message) => ({
      role: message.role,
      text: message.text,
      commands: message.proposals.map((proposal) => proposal.command),
    }));
}

/**
 * Turns a proposal into something the normal command form can open. The risk
 * level travels with it so the entry is saved as the level the user reviewed,
 * not as whatever the text re-derives to on its own.
 */
export function proposalToInput(proposal: CommandProposal): CommandInput {
  return emptyCommandInput({
    title: proposal.title,
    content: proposal.command,
    description: proposal.why,
    kind: proposal.kind,
    shell: proposal.shell,
    riskLevel: proposal.riskLevel,
  });
}

/** Unique enough for React keys within one in-memory conversation. */
export function messageId(role: AssistantRole, sequence: number): string {
  return `${role}-${sequence}`;
}

/**
 * The last thing the user asked, used by Retry. A failed exchange leaves the
 * question on screen, so retrying resends that message with the conversation
 * that came before it.
 */
export function lastQuestion(
  messages: AssistantMessage[],
): { message: string; history: AssistantMessage[] } | null {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const candidate = messages[index];
    if (candidate.role === "user") {
      return { message: candidate.text, history: messages.slice(0, index) };
    }
  }
  return null;
}
