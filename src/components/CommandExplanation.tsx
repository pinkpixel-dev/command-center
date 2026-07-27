import { useState } from "react";
import { Copy, RefreshCw, Sparkles } from "lucide-react";

import { useExplanation } from "../hooks/useExplanation";
import { relativeTime } from "../lib/format";
import type { Explanation, ExplanationPreview } from "../lib/types";
import { Button } from "./ui/Button";
import { RiskBadge } from "./ui/RiskBadge";

export interface CommandExplanationProps {
  commandId: number;
  /** AI is on, a key is stored, and the credential manager answered. */
  ready: boolean;
  onCopy: (text: string) => void;
}

type Detail = "quick" | "detailed";

/**
 * The Explain section inside the full entry dialog. One structured result fills
 * both the Quick and the Detailed view, so switching between them never costs
 * another request.
 */
export function CommandExplanation({ commandId, ready, onCopy }: CommandExplanationProps) {
  const { view, loading, generating, error, explain } = useExplanation(commandId, ready);
  const [detail, setDetail] = useState<Detail>("quick");

  if (!ready) return null;

  return (
    <section className="explain" aria-labelledby={`explain-heading-${commandId}`}>
      <div className="explain__head">
        <p className="card__section-label" id={`explain-heading-${commandId}`}>
          Explanation
        </p>
        {view && (
          <div className="explain__views" role="group" aria-label="Explanation detail">
            {(["quick", "detailed"] as const).map((option) => (
              <button
                key={option}
                type="button"
                className={`explain__view${detail === option ? " is-active" : ""}`}
                aria-pressed={detail === option}
                onClick={() => setDetail(option)}
              >
                {option === "quick" ? "Quick" : "Detailed"}
              </button>
            ))}
          </div>
        )}
      </div>

      {loading && <p className="explain__note">Checking for a saved explanation…</p>}

      {!loading && !view && (
        <div className="explain__empty">
          <p className="explain__note">
            Ask AI what this entry does. The explanation is saved here.
          </p>
          <Button variant="secondary" size="sm" loading={generating} onClick={explain}>
            <Sparkles size={15} aria-hidden="true" />
            Explain this entry
          </Button>
        </div>
      )}

      {view && (
        <>
          {view.stale && (
            <p className="explain__stale" role="note">
              This entry changed after the explanation was written, so parts of it may no longer
              be accurate. Please refresh the explanation to see the latest version.
            </p>
          )}

          <ExplanationBody explanation={view.explanation} detail={detail} onCopy={onCopy} />

          <div className="explain__foot">
            <p className="explain__origin">
              Written by {view.model}, {relativeTime(view.generatedAt)}. AI-generated results may be
              inaccurate. Check anything you have not run before.
            </p>
            <Button variant="ghost" size="sm" loading={generating} onClick={explain}>
              <RefreshCw size={14} aria-hidden="true" />
              Refresh
            </Button>
          </div>
        </>
      )}

      {error && (
        <p className="explain__error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

interface ExplanationBodyProps {
  explanation: Explanation;
  detail: Detail;
  onCopy: (text: string) => void;
}

function ExplanationBody({ explanation, detail, onCopy }: ExplanationBodyProps) {
  const { safety } = explanation;

  return (
    <div className="explain__body">
      <p className="explain__summary">{explanation.summary}</p>

      {detail === "detailed" && (
        <>
          <ExplanationList
            label="What each part does"
            items={explanation.flags.map((flag) => ({
              key: flag.flag,
              term: flag.flag,
              detail: flag.meaning,
            }))}
          />

          <ExplanationList
            label="Step by step"
            items={explanation.pipeline.map((stage) => ({
              key: stage.stage,
              term: stage.stage,
              detail: stage.purpose,
            }))}
          />

          <ExplanationNotes label="What it changes" items={explanation.sideEffects} />

          <div className="explain__safety">
            <p className="card__section-label">Safety</p>
            <RiskBadge risk={safety.level} reasons={safety.localReasons} />
            {safety.localReasons.length > 0 && (
              <ul className="explain__reasons">
                {safety.localReasons.map((reason) => (
                  <li key={reason}>
                    <span className="explain__source">Command Center</span> {reason}
                  </li>
                ))}
              </ul>
            )}
            {safety.aiReasons.length > 0 && (
              <ul className="explain__reasons">
                {safety.aiReasons.map((reason) => (
                  <li key={reason}>
                    <span className="explain__source explain__source--ai">OpenAI</span> {reason}
                  </li>
                ))}
              </ul>
            )}
          </div>

          {explanation.previewCommand && (
            <PreviewProposal preview={explanation.previewCommand} onCopy={onCopy} />
          )}

          <ExplanationNotes label="Assumed while writing this" items={explanation.assumptions} />
          <ExplanationNotes label="Worth knowing" items={explanation.caveats} />
        </>
      )}
    </div>
  );
}

interface PreviewProposalProps {
  preview: ExplanationPreview;
  onCopy: (text: string) => void;
}

/** A suggested command is never run and never saved from here, only copied. */
function PreviewProposal({ preview, onCopy }: PreviewProposalProps) {
  return (
    <div className="explain__preview">
      <p className="card__section-label">See what it would do first</p>
      <pre className="code explain__preview-code" tabIndex={0}>
        <code>{preview.command}</code>
      </pre>
      <div className="explain__preview-foot">
        <RiskBadge risk={preview.riskLevel} reasons={preview.riskReasons} />
        <Button variant="ghost" size="sm" onClick={() => onCopy(preview.command)}>
          <Copy size={14} aria-hidden="true" />
          Copy preview
        </Button>
      </div>
      {preview.riskReasons.length > 0 && (
        <ul className="explain__reasons">
          {preview.riskReasons.map((reason) => (
            <li key={reason}>
              <span className="explain__source">Command Center</span> {reason}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

interface ExplanationListProps {
  label: string;
  items: { key: string; term: string; detail: string }[];
}

function ExplanationList({ label, items }: ExplanationListProps) {
  if (items.length === 0) return null;

  return (
    <div className="explain__group">
      <p className="card__section-label">{label}</p>
      <dl className="explain__terms">
        {items.map((item) => (
          <div key={item.key}>
            <dt className="mono">{item.term}</dt>
            <dd>{item.detail}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

function ExplanationNotes({ label, items }: { label: string; items: string[] }) {
  if (items.length === 0) return null;

  return (
    <div className="explain__group">
      <p className="card__section-label">{label}</p>
      <ul className="explain__notes">
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}
