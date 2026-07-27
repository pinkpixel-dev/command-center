/**
 * The assistant conversation as the panel holds it: in memory, for this window
 * session only. Nothing here writes to the database, and closing the panel or
 * turning AI off is the whole of the delete story.
 */

import { emptyCommandInput } from "./types";
import type { AssistantTurn, CommandInput, CommandProposal } from "./types";

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
  /** Set when this message is the failed half of an exchange. */
  failed?: boolean;
}

/** What the conversation is about. `null` is general command help. */
export interface AssistantContext {
  commandId: number;
  title: string;
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
