import { useAssistant } from "../../hooks/useAssistant";
import type { AssistantContext } from "../../lib/assistant";
import type { CommandProposal } from "../../lib/types";
import { AssistantPanel } from "./AssistantPanel";

export interface AssistantDockProps {
  open: boolean;
  context: AssistantContext | null;
  /** AI is on, a key is stored, and the credential manager answered. */
  ready: boolean;
  model: string | null;
  onClose: () => void;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

/**
 * Holds the conversation for as long as the window is open, so closing the
 * panel to look something up does not throw the thread away. Turning AI off or
 * moving to another entry does clear it, and nothing is ever written to disk.
 */
export function AssistantDock({
  open,
  context,
  ready,
  model,
  onClose,
  onCopy,
  onReview,
}: AssistantDockProps) {
  const assistant = useAssistant(context, ready);

  if (!open || !ready) return null;

  return (
    <AssistantPanel
      context={context}
      model={model}
      messages={assistant.messages}
      sending={assistant.sending}
      error={assistant.error}
      cancelled={assistant.cancelled}
      retryable={assistant.retryable}
      onSend={assistant.send}
      onCancel={assistant.cancel}
      onRetry={assistant.retry}
      onClear={assistant.clear}
      onClose={onClose}
      onCopy={onCopy}
      onReview={onReview}
    />
  );
}
