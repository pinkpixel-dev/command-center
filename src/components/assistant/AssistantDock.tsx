import { useCallback, useEffect, useState } from "react";

import { useAssistant } from "../../hooks/useAssistant";
import { useErrorAnalysis } from "../../hooks/useErrorAnalysis";
import type { AssistantContext } from "../../lib/assistant";
import { nextRequestId } from "../../lib/request-id";
import type { CommandProposal } from "../../lib/types";
import { AssistantPanel } from "./AssistantPanel";
import type { AssistantMode } from "./AssistantPanel";
import { ErrorComposer } from "./ErrorComposer";

export interface AssistantDockProps {
  open: boolean;
  context: AssistantContext | null;
  /** Which surface to open on, so the palette can go straight to the paste box. */
  initialMode: AssistantMode;
  /** AI is on, a key is stored, and the credential manager answered. */
  ready: boolean;
  model: string | null;
  onClose: () => void;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
  /** A finished analysis becomes the conversation the panel is now holding. */
  onErrorAnalyzed: (context: AssistantContext) => void;
}

/**
 * Holds the conversation for as long as the window is open, so closing the
 * panel to look something up does not throw the thread away. Turning AI off or
 * changing subject does clear it, and nothing is ever written to disk.
 *
 * A pasted error becomes a conversation rather than a one-off answer: the
 * analysis is the first message, and follow-up questions carry the paste with
 * them so "which line said that?" has something to look at.
 */
export function AssistantDock({
  open,
  context,
  initialMode,
  ready,
  model,
  onClose,
  onCopy,
  onReview,
  onErrorAnalyzed,
}: AssistantDockProps) {
  const [mode, setMode] = useState<AssistantMode>(initialMode);
  const assistant = useAssistant(context, ready);
  const analysis = useErrorAnalysis(ready);

  // Opening from the palette's "Read a terminal error" lands on the paste box.
  useEffect(() => {
    if (open) setMode(initialMode);
  }, [open, initialMode]);

  const analyze = useCallback(
    (output: string) => {
      void analysis.analyze(output).then((result) => {
        if (!result) return;
        onErrorAnalyzed({
          kind: "error",
          sessionId: nextRequestId(),
          output,
          analysis: result,
          // Short and stable. The summary is the first thing in the
          // transcript, so repeating it in the header just crowds it.
          title: "Terminal error",
        });
        analysis.discard();
        setMode("chat");
      });
    },
    [analysis, onErrorAnalyzed],
  );

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
      mode={mode}
      canSwitchMode={assistant.messages.length === 0 && !assistant.sending}
      onModeChange={setMode}
      pasteView={
        <ErrorComposer
          plan={analysis.plan}
          preparing={analysis.preparing}
          analyzing={analysis.analyzing}
          error={analysis.error}
          cancelled={analysis.cancelled}
          onPrepare={analysis.prepare}
          onAnalyze={analyze}
          onCancel={analysis.cancel}
          onDiscard={analysis.discard}
        />
      }
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
