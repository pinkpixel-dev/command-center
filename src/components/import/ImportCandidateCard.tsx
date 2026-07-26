import { AlertTriangle, Copy, Scissors, Sparkles, Trash2 } from "lucide-react";

import { kindLabel } from "../../lib/format";
import type { CandidateDraft, DuplicateAction } from "../../lib/import";
import { COMMAND_KINDS } from "../../lib/types";
import type { CommandKind } from "../../lib/types";
import { Button } from "../ui/Button";
import { RiskBadge } from "../ui/RiskBadge";
import { TagInput } from "../ui/TagInput";

export interface ImportCandidateCardProps {
  draft: CandidateDraft;
  index: number;
  canMergeUp: boolean;
  tagSuggestions: string[];
  onChange: (changes: Partial<CandidateDraft>) => void;
  onContentCommitted: () => void;
  onSplit: () => void;
  onMergeUp: () => void;
  onRemove: () => void;
}

const DUPLICATE_ACTIONS: { value: DuplicateAction; label: string; hint: string }[] = [
  { value: "skip", label: "Skip", hint: "Leave the saved entry alone" },
  { value: "merge", label: "Merge", hint: "Keep the saved command, add these tags and notes" },
  { value: "replace", label: "Replace", hint: "Overwrite the saved entry with this one" },
  { value: "create", label: "Keep both", hint: "Save this as a second entry" },
];

export function ImportCandidateCard({
  draft,
  index,
  canMergeUp,
  tagSuggestions,
  onChange,
  onContentCommitted,
  onSplit,
  onMergeUp,
  onRemove,
}: ImportCandidateCardProps) {
  const checkboxId = `candidate-${draft.id}-selected`;
  const splittable = draft.content.trim().split("\n").filter((line) => line.trim()).length > 1;

  return (
    <article
      className={`candidate${draft.selected ? " is-selected" : ""}${
        draft.looksLikeOutput ? " is-output" : ""
      }`}
      data-risk={draft.riskLevel}
    >
      <div className="candidate__head">
        <input
          id={checkboxId}
          type="checkbox"
          className="checkbox__input"
          checked={draft.selected}
          onChange={(event) => onChange({ selected: event.target.checked })}
          aria-label={`Import ${draft.title || "this entry"}`}
        />

        <input
          className="input candidate__title"
          value={draft.title}
          onChange={(event) => onChange({ title: event.target.value })}
          placeholder="Title"
          aria-label={`Title for entry ${index + 1}`}
        />

        {draft.riskLevel !== "safe" && (
          <RiskBadge risk={draft.riskLevel} reasons={draft.riskReasons} />
        )}
      </div>

      {draft.looksLikeOutput && (
        <p className="candidate__flag" role="note">
          <AlertTriangle size={14} aria-hidden="true" />
          Looks like terminal output. {draft.outputReason}
        </p>
      )}

      {draft.duplicate && (
        <div className="candidate__flag candidate__flag--duplicate">
          <p>
            <Copy size={14} aria-hidden="true" />
            Already saved as "{draft.duplicate.title}"
          </p>
          <fieldset className="radio-row">
            <legend className="visually-hidden">What to do about the duplicate</legend>
            {DUPLICATE_ACTIONS.map((action) => (
              <label key={action.value} className="radio" title={action.hint}>
                <input
                  type="radio"
                  name={`duplicate-${draft.id}`}
                  value={action.value}
                  checked={draft.duplicateAction === action.value}
                  onChange={() => onChange({ duplicateAction: action.value })}
                />
                {action.label}
              </label>
            ))}
          </fieldset>
        </div>
      )}

      {(draft.aiRiskReasons.length > 0 || draft.aiNotes) && (
        <div className="candidate__flag candidate__flag--ai" role="note">
          <p>
            <Sparkles size={14} aria-hidden="true" />
            <span className="candidate__ai-label">From the model, and it may be wrong</span>
          </p>
          {draft.aiRiskReasons.length > 0 && (
            <p className="candidate__ai-body">Risk notes: {draft.aiRiskReasons.join("; ")}</p>
          )}
          {draft.aiNotes && <p className="candidate__ai-body">Unsure about: {draft.aiNotes}</p>}
        </div>
      )}

      {draft.repeatedInDocument && !draft.duplicate && (
        <p className="candidate__flag" role="note">
          <Copy size={14} aria-hidden="true" />
          This command appears earlier in the same document.
        </p>
      )}

      <textarea
        className="input input--textarea input--mono candidate__content"
        value={draft.content}
        onChange={(event) => onChange({ content: event.target.value })}
        onBlur={onContentCommitted}
        rows={Math.min(8, Math.max(2, draft.content.split("\n").length))}
        spellCheck={false}
        aria-label={`Command for entry ${index + 1}`}
      />

      <input
        className="input candidate__description"
        value={draft.description}
        onChange={(event) => onChange({ description: event.target.value })}
        placeholder="Description (optional)"
        aria-label={`Description for entry ${index + 1}`}
      />

      <div className="candidate__row">
        <label className="select-inline">
          <span className="visually-hidden">{`Type for entry ${index + 1}`}</span>
          <select
            className="input input--select input--compact"
            value={draft.kind}
            onChange={(event) => onChange({ kind: event.target.value as CommandKind })}
          >
            {COMMAND_KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {kindLabel(kind)}
              </option>
            ))}
          </select>
        </label>

        <div className="candidate__tags">
          <TagInput
            label={`Tags for entry ${index + 1}`}
            value={draft.tags}
            suggestions={tagSuggestions}
            onChange={(tags) => onChange({ tags })}
          />
        </div>
      </div>

      <div className="candidate__meta">
        {draft.headingPath.length > 0 && (
          <span className="candidate__source">{draft.headingPath.join(" › ")}</span>
        )}
        {draft.sourceLine > 0 && (
          <span className="candidate__source">line {draft.sourceLine}</span>
        )}
        {draft.droppedOutputLines > 0 && (
          <span className="candidate__source">
            {draft.droppedOutputLines} output {draft.droppedOutputLines === 1 ? "line" : "lines"}{" "}
            removed
          </span>
        )}
        {draft.variables.length > 0 && (
          <span className="candidate__source">
            placeholders: {draft.variables.map((name) => `{{${name}}}`).join(" ")}
          </span>
        )}
      </div>

      <div className="candidate__actions">
        {splittable && (
          <Button variant="ghost" size="sm" onClick={onSplit} title="One entry per line">
            <Scissors size={14} aria-hidden="true" />
            Split lines
          </Button>
        )}
        {canMergeUp && (
          <Button variant="ghost" size="sm" onClick={onMergeUp} title="Combine with the entry above">
            Merge up
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="candidate__remove"
          onClick={onRemove}
          aria-label={`Remove ${draft.title || `entry ${index + 1}`} from the import`}
        >
          <Trash2 size={14} aria-hidden="true" />
          Remove
        </Button>
      </div>
    </article>
  );
}
