import { useImportSession } from "../../hooks/useImportSession";
import type { Collection } from "../../lib/types";
import { ImportDisclosure } from "./ImportDisclosure";
import { ImportDone } from "./ImportDone";
import { ImportReview } from "./ImportReview";
import { ImportSource } from "./ImportSource";

export interface ImportViewProps {
  collections: Collection[];
  tagSuggestions: string[];
  onOpenLibrary: () => void;
  onImported: () => void;
}

/** The whole import flow: pick a source, review what was found, import it. */
export function ImportView({
  collections,
  tagSuggestions,
  onOpenLibrary,
  onImported,
}: ImportViewProps) {
  const session = useImportSession();

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
          initialText={session.pending?.sourceName ? "" : (session.pending?.content ?? "")}
          onReadFile={(read) => void session.prepareFile(read)}
          onReadText={(content) => void session.prepareText(content)}
        />
      )}

      {session.stage === "disclose" && session.plan && (
        <ImportDisclosure
          plan={session.plan}
          sourceName={session.pending?.sourceName ?? null}
          busy={session.busy}
          onSend={() => void session.send()}
          onCancel={session.cancelSend}
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
