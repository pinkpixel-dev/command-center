import { useState } from "react";
import { ArrowLeft, ShieldCheck, Upload } from "lucide-react";

import { describeOutbound, describeRedactions, groupRedactions } from "../../lib/ai-import";
import type { AiImportPlan } from "../../lib/ai-import";
import { Button } from "../ui/Button";

export interface ImportDisclosureProps {
  plan: AiImportPlan;
  sourceName: string | null;
  busy: boolean;
  onSend: () => void;
  onCancel: () => void;
}

/**
 * The step between reading a document and sending it. Nothing has left the
 * machine when this renders: the size, the model, and the redactions all come
 * from the local pass Rust already ran.
 */
export function ImportDisclosure({
  plan,
  sourceName,
  busy,
  onSend,
  onCancel,
}: ImportDisclosureProps) {
  const [showFindings, setShowFindings] = useState(false);
  const redactions = describeRedactions(plan);
  const groups = groupRedactions(plan);

  return (
    <section className="disclosure" aria-labelledby="disclosure-title">
      <div className="disclosure__head">
        <Upload size={18} aria-hidden="true" />
        <div>
          <h2 id="disclosure-title" className="disclosure__title">
            Ready to send
          </h2>
          <p className="disclosure__source">
            {sourceName ?? "Pasted text"} · {plan.lineCount}{" "}
            {plan.lineCount === 1 ? "line" : "lines"}
          </p>
        </div>
      </div>

      <p className="disclosure__outbound">{describeOutbound(plan)}</p>

      <div className="disclosure__facts">
        <p>
          <ShieldCheck size={14} aria-hidden="true" />
          {redactions ?? "No likely secrets were found in this document."}
        </p>
      </div>

      {groups.length > 0 && (
        <div className="disclosure__findings">
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={showFindings}
            aria-controls="disclosure-findings"
            onClick={() => setShowFindings((open) => !open)}
          >
            {showFindings ? "Hide what was replaced" : "Review what was replaced"}
          </Button>

          {showFindings && (
            <ul id="disclosure-findings" className="disclosure__list">
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

      <div className="disclosure__actions">
        <Button variant="ghost" onClick={onCancel} disabled={busy}>
          <ArrowLeft size={15} aria-hidden="true" />
          Back
        </Button>
        <Button variant="primary" onClick={onSend} loading={busy}>
          <Upload size={15} aria-hidden="true" />
          Send to OpenAI
        </Button>
      </div>
    </section>
  );
}
