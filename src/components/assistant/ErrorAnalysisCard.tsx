import { CircleHelp } from "lucide-react";

import type { AnalysisConfidence, ErrorAnalysis } from "../../lib/types";

export interface ErrorAnalysisCardProps {
  analysis: ErrorAnalysis;
}

const CONFIDENCE_LABELS: Record<AnalysisConfidence, string> = {
  high: "The output says this outright",
  medium: "Reading between the lines",
  low: "Largely a guess",
};

/**
 * The structured half of an error analysis. Proposed commands are not rendered
 * here: they are the message's proposals, so they go through the same card, the
 * same risk badge, and the same Review and save as every other suggestion.
 *
 * Confidence is shown as text rather than only as a colour, because "largely a
 * guess" is the most useful thing on this card when it is true.
 */
export function ErrorAnalysisCard({ analysis }: ErrorAnalysisCardProps) {
  return (
    <div className="analysis">
      <p className="analysis__confidence" data-confidence={analysis.confidence}>
        <CircleHelp size={13} aria-hidden="true" />
        {CONFIDENCE_LABELS[analysis.confidence]}
      </p>

      {analysis.relevantLines.length > 0 && (
        <div className="analysis__group">
          <p className="card__section-label">Where it went wrong</p>
          <ul className="analysis__lines">
            {analysis.relevantLines.map((line) => (
              <li key={`${line.line ?? "x"}-${line.quote}`}>
                {line.line !== null && <span className="analysis__line-no">{line.line}</span>}
                <code className="analysis__quote">{line.quote}</code>
                {line.why && <span className="analysis__why">{line.why}</span>}
              </li>
            ))}
          </ul>
        </div>
      )}

      {analysis.checks.length > 0 && (
        <div className="analysis__group">
          <p className="card__section-label">What to check next</p>
          <ol className="analysis__checks">
            {analysis.checks.map((check) => (
              <li key={check.check}>
                <span className="analysis__check">{check.check}</span>
                {check.why && <span className="analysis__why">{check.why}</span>}
              </li>
            ))}
          </ol>
        </div>
      )}

      {analysis.uncertainty && (
        <p className="analysis__uncertainty" role="note">
          <strong>Not shown here:</strong> {analysis.uncertainty}
        </p>
      )}
    </div>
  );
}
