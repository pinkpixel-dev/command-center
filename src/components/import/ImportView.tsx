import { useMemo } from "react";

import { useImportSession } from "../../hooks/useImportSession";
import type { Collection } from "../../lib/types";
import { ImportDone } from "./ImportDone";
import { ImportReview } from "./ImportReview";
import { ImportSource } from "./ImportSource";

export interface ImportViewProps {
  collections: Collection[];
  tagSuggestions: string[];
  defaultCollectionId: number | null;
  onOpenLibrary: () => void;
  onImported: () => void;
}

/** The whole import flow: pick a source, review what was found, import it. */
export function ImportView({
  collections,
  tagSuggestions,
  defaultCollectionId,
  onOpenLibrary,
  onImported,
}: ImportViewProps) {
  // A stable array, so the session's loaders keep their identity between renders.
  const defaultCollectionIds = useMemo(
    () => (defaultCollectionId !== null ? [defaultCollectionId] : []),
    [defaultCollectionId],
  );

  const session = useImportSession(defaultCollectionIds);

  const runImport = async () => {
    await session.runImport();
    onImported();
  };

  return (
    <div className="import">
      {session.error && (
        <div className="form__error import__error" role="alert">
          <span>{session.error}</span>
          <button type="button" className="link-button" onClick={session.dismissError}>
            Dismiss
          </button>
        </div>
      )}

      {session.stage === "source" && (
        <ImportSource
          busy={session.busy}
          onScanFile={(path) => void session.scanFile(path)}
          onScanText={(content) => void session.scanText(content)}
        />
      )}

      {session.stage === "review" && session.preview && (
        <ImportReview
          preview={session.preview}
          drafts={session.drafts}
          counts={session.counts}
          collections={collections}
          tagSuggestions={tagSuggestions}
          busy={session.busy}
          onUpdate={session.update}
          onContentCommitted={(id) => void session.reanalyze(id)}
          onSelectAll={session.setSelectionForAll}
          onCollectionForAll={session.setCollectionForAll}
          onSplit={(id) => void session.split(id)}
          onMergeUp={(id) => void session.mergeWithPrevious(id)}
          onRemove={session.remove}
          onImport={() => void runImport()}
          onBack={session.reset}
        />
      )}

      {session.stage === "done" && session.summary && (
        <ImportDone
          summary={session.summary}
          onImportAnother={session.reset}
          onOpenLibrary={onOpenLibrary}
        />
      )}
    </div>
  );
}
