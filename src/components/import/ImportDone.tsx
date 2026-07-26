import { CheckCircle2, Library, Upload } from "lucide-react";

import { summarize } from "../../lib/import";
import type { ImportSummary } from "../../lib/import";
import { Button } from "../ui/Button";

export interface ImportDoneProps {
  summary: ImportSummary;
  onImportAnother: () => void;
  onOpenLibrary: () => void;
}

export function ImportDone({ summary, onImportAnother, onOpenLibrary }: ImportDoneProps) {
  const total = summary.created + summary.replaced + summary.merged;

  return (
    <div className="import-done">
      <div className="import-done__headline">
        <CheckCircle2 size={20} aria-hidden="true" />
        <div>
          <h2>{total > 0 ? "Import finished" : "Nothing was added"}</h2>
          <p className="field__hint">{summarize(summary)}</p>
        </div>
      </div>

      <dl className="card__facts">
        <div>
          <dt>Added</dt>
          <dd>{summary.created}</dd>
        </div>
        {summary.replaced > 0 && (
          <div>
            <dt>Replaced</dt>
            <dd>{summary.replaced}</dd>
          </div>
        )}
        {summary.merged > 0 && (
          <div>
            <dt>Merged</dt>
            <dd>{summary.merged}</dd>
          </div>
        )}
        {summary.skipped > 0 && (
          <div>
            <dt>Skipped</dt>
            <dd>{summary.skipped}</dd>
          </div>
        )}
      </dl>

      {summary.failures.length > 0 && (
        <div className="import-done__failures" role="alert">
          <h3>These could not be saved</h3>
          <ul>
            {summary.failures.map((failure, index) => (
              <li key={`${failure.title}-${index}`}>
                <strong>{failure.title}</strong> — {failure.message}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="import-done__actions">
        <Button variant="primary" onClick={onOpenLibrary}>
          <Library size={15} aria-hidden="true" />
          Open the library
        </Button>
        <Button variant="secondary" onClick={onImportAnother}>
          <Upload size={15} aria-hidden="true" />
          Import something else
        </Button>
      </div>
    </div>
  );
}
