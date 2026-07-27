import { useState } from "react";
import { ArrowLeft, ShieldCheck, Terminal, Upload, X } from "lucide-react";

import { describeOutbound, describeRedactions, groupRedactions } from "../../lib/outbound";
import type { OutboundPlan } from "../../lib/outbound";
import { Button } from "../ui/Button";

export interface ErrorComposerProps {
  plan: OutboundPlan | null;
  preparing: boolean;
  analyzing: boolean;
  error: string | null;
  cancelled: boolean;
  onPrepare: (output: string) => void;
  onAnalyze: (output: string) => void;
  onCancel: () => void;
  onDiscard: () => void;
}

/**
 * Paste terminal output, see exactly what would leave the machine, then send
 * it. The disclosure is the same one Import shows, because it is the same
 * question: this is your text, here is how much of it goes out, and here is
 * every likely secret that was taken out of it first.
 */
export function ErrorComposer({
  plan,
  preparing,
  analyzing,
  error,
  cancelled,
  onPrepare,
  onAnalyze,
  onCancel,
  onDiscard,
}: ErrorComposerProps) {
  const [output, setOutput] = useState("");
  const [showFindings, setShowFindings] = useState(false);
  const empty = output.trim().length === 0;

  if (plan) {
    const redactions = describeRedactions(plan);
    const groups = groupRedactions(plan);

    return (
      <div className="assistant__paste">
        <div className="disclosure disclosure--inline">
          <div className="disclosure__head">
            <Upload size={16} aria-hidden="true" />
            <div>
              <h3 className="disclosure__title">Ready to send</h3>
              <p className="disclosure__source">
                {plan.lineCount} {plan.lineCount === 1 ? "line" : "lines"} of output
              </p>
            </div>
          </div>

          <p className="disclosure__outbound">{describeOutbound(plan)}</p>

          <div className="disclosure__facts">
            <p>
              <ShieldCheck size={14} aria-hidden="true" />
              {redactions ?? "No likely secrets were found in this output."}
            </p>
            <p className="field__hint">
              Secret detection is a safety net, not a guarantee. Read the output first if it holds
              anything you would not paste into a support ticket.
            </p>
          </div>

          {groups.length > 0 && (
            <div className="disclosure__findings">
              <Button
                variant="ghost"
                size="sm"
                aria-expanded={showFindings}
                aria-controls="error-findings"
                onClick={() => setShowFindings((open) => !open)}
              >
                {showFindings ? "Hide what was replaced" : "Review what was replaced"}
              </Button>

              {showFindings && (
                <ul id="error-findings" className="disclosure__list">
                  {groups.map((group) => (
                    <li key={group.kind}>
                      <span className="disclosure__kind">{group.label}</span>
                      <span className="disclosure__lines">
                        {group.lines.length === 1
                          ? `line ${group.lines[0]}`
                          : `lines ${group.lines.join(", ")}`}
                      </span>
                      <code className="disclosure__placeholder">{group.placeholder}</code>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}

          <p className="field__hint">
            You get a likely cause and some checks to try, not a verdict. Any command that comes
            back is checked against Command Center&apos;s own rules and never runs on its own.
          </p>

          <div className="disclosure__actions">
            <Button variant="ghost" onClick={onDiscard} disabled={analyzing}>
              <ArrowLeft size={15} aria-hidden="true" />
              Back
            </Button>
            {analyzing ? (
              <Button variant="secondary" onClick={onCancel}>
                <X size={15} aria-hidden="true" />
                Stop
              </Button>
            ) : (
              <Button variant="primary" onClick={() => onAnalyze(output)}>
                <Upload size={15} aria-hidden="true" />
                Send to OpenAI
              </Button>
            )}
          </div>
        </div>

        {cancelled && (
          <p className="assistant__note assistant__note--stopped" role="status">
            Stopped. Nothing was analyzed.
          </p>
        )}

        {error && (
          <p className="assistant__error" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="assistant__paste">
      <div className="assistant__empty">
        <Terminal size={20} aria-hidden="true" />
        <p className="assistant__empty-title">Paste what your terminal printed</p>
        <p className="assistant__note">
          The part around the failure is usually enough. You will see exactly what would be sent
          before anything leaves this machine.
        </p>
      </div>

      <label className="field" htmlFor="error-output">
        <span className="field__label">Terminal output</span>
        <textarea
          id="error-output"
          className="input assistant__paste-input"
          rows={8}
          value={output}
          placeholder={"$ npm run build\nError: Cannot find module 'vite'"}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => setOutput(event.target.value)}
        />
      </label>

      {error && (
        <p className="assistant__error" role="alert">
          {error}
        </p>
      )}

      <div className="assistant__paste-actions">
        <Button
          variant="primary"
          disabled={empty}
          loading={preparing}
          onClick={() => onPrepare(output)}
        >
          <ShieldCheck size={15} aria-hidden="true" />
          Check what would be sent
        </Button>
      </div>
    </div>
  );
}
