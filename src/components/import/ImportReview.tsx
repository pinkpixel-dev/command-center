import { ArrowLeft, Download } from "lucide-react";

import type { CandidateDraft, DraftCounts, ImportPreview } from "../../lib/import";
import type { Collection } from "../../lib/types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { ImportCandidateCard } from "./ImportCandidateCard";
import { FileQuestion } from "lucide-react";

export interface ImportReviewProps {
  preview: ImportPreview;
  drafts: CandidateDraft[];
  counts: DraftCounts;
  collections: Collection[];
  tagSuggestions: string[];
  busy: boolean;
  onUpdate: (id: string, changes: Partial<CandidateDraft>) => void;
  onContentCommitted: (id: string) => void;
  onSelectAll: (selected: boolean) => void;
  onCollectionForAll: (collectionIds: number[]) => void;
  onSplit: (id: string) => void;
  onMergeUp: (id: string) => void;
  onRemove: (id: string) => void;
  onImport: () => void;
  onBack: () => void;
}

export function ImportReview({
  preview,
  drafts,
  counts,
  collections,
  tagSuggestions,
  busy,
  onUpdate,
  onContentCommitted,
  onSelectAll,
  onCollectionForAll,
  onSplit,
  onMergeUp,
  onRemove,
  onImport,
  onBack,
}: ImportReviewProps) {
  // Every draft carries the same list while the bulk picker drives it.
  const sharedCollection = drafts.length > 0 ? (drafts[0].collectionIds[0] ?? "") : "";

  return (
    <div className="import-review">
      <div className="import-review__bar">
        <div className="import-review__summary">
          <strong>
            {counts.selected} of {counts.total} selected
          </strong>
          <span className="field__hint">
            {preview.sourceName ? `${preview.sourceName} · ` : ""}
            {preview.stats.blocksFound} blocks found
            {counts.output > 0 ? ` · ${counts.output} look like output` : ""}
            {counts.duplicates > 0 ? ` · ${counts.duplicates} already saved` : ""}
          </span>
        </div>

        <div className="import-review__controls">
          <Button variant="ghost" size="sm" onClick={() => onSelectAll(true)}>
            Select all
          </Button>
          <Button variant="ghost" size="sm" onClick={() => onSelectAll(false)}>
            Select none
          </Button>

          {collections.length > 0 && (
            <label className="select-inline">
              <span className="visually-hidden">Collection for every entry</span>
              <select
                className="input input--select input--compact"
                value={String(sharedCollection)}
                onChange={(event) =>
                  onCollectionForAll(event.target.value ? [Number(event.target.value)] : [])
                }
              >
                <option value="">No collection</option>
                {collections.map((collection) => (
                  <option key={collection.id} value={collection.id}>
                    {collection.name}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
      </div>

      {preview.suggestedCollection && collections.length === 0 && (
        <p className="field__hint import-review__suggestion">
          This document looks like it belongs in a collection called "{preview.suggestedCollection}".
          Create it from the sidebar and it will show up here.
        </p>
      )}

      {drafts.length === 0 ? (
        <EmptyState
          icon={FileQuestion}
          title="Nothing to import"
          body="No code blocks, indented commands, or prompt lines turned up in that document."
          action={
            <Button variant="secondary" onClick={onBack}>
              Try another document
            </Button>
          }
        />
      ) : (
        <ul className="candidate-list">
          {drafts.map((draft, index) => (
            <li key={draft.id}>
              <ImportCandidateCard
                draft={draft}
                index={index}
                canMergeUp={index > 0}
                tagSuggestions={tagSuggestions}
                onChange={(changes) => onUpdate(draft.id, changes)}
                onContentCommitted={() => onContentCommitted(draft.id)}
                onSplit={() => onSplit(draft.id)}
                onMergeUp={() => onMergeUp(draft.id)}
                onRemove={() => onRemove(draft.id)}
              />
            </li>
          ))}
        </ul>
      )}

      <div className="import-review__footer">
        <Button variant="ghost" onClick={onBack}>
          <ArrowLeft size={15} aria-hidden="true" />
          Start over
        </Button>
        <div className="import-review__footer-actions">
          {counts.willSkip > 0 && (
            <span className="field__hint">
              {counts.willSkip} duplicate {counts.willSkip === 1 ? "entry" : "entries"} set to skip
            </span>
          )}
          <Button
            variant="primary"
            onClick={onImport}
            loading={busy}
            disabled={counts.selected === 0}
          >
            <Download size={15} aria-hidden="true" />
            Import {counts.selected} {counts.selected === 1 ? "entry" : "entries"}
          </Button>
        </div>
      </div>
    </div>
  );
}
