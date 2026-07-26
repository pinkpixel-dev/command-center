import { useMemo, useState } from "react";
import { Copy, ExternalLink, Pencil, Trash2 } from "lucide-react";

import { kindLabel, relativeTime, renderTemplate } from "../lib/format";
import type { CommandEntry } from "../lib/types";
import { Button } from "./ui/Button";
import { Modal } from "./ui/Modal";
import { RiskBadge } from "./ui/RiskBadge";

export interface CommandDetailsDialogProps {
  entry: CommandEntry;
  open: boolean;
  onClose: () => void;
  onCopy: (text: string) => void;
  onEdit: () => void;
  onDelete: () => void;
  onOpenSource: (url: string) => void;
}

export function CommandDetailsDialog({
  entry,
  open,
  onClose,
  onCopy,
  onEdit,
  onDelete,
  onOpenSource,
}: CommandDetailsDialogProps) {
  const [values, setValues] = useState<Record<string, string>>({});
  const resolved = useMemo(
    () => (entry.variables.length > 0 ? renderTemplate(entry.content, values) : entry.content),
    [entry.content, entry.variables.length, values],
  );

  const closeThen = (action: () => void) => {
    onClose();
    action();
  };

  return (
    <Modal
      open={open}
      size="xl"
      mobileFullscreen
      title={entry.title}
      description={`${kindLabel(entry.kind)}${entry.shell ? ` · ${entry.shell}` : ""}`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={() => closeThen(onDelete)}>
            <Trash2 size={15} aria-hidden="true" />
            Delete
          </Button>
          <Button variant="ghost" onClick={() => closeThen(onEdit)}>
            <Pencil size={15} aria-hidden="true" />
            Edit
          </Button>
          <Button variant="primary" onClick={() => onCopy(resolved)}>
            <Copy size={15} aria-hidden="true" />
            Copy
          </Button>
        </>
      }
    >
      <div className="entry-dialog">
        <div className="entry-dialog__meta">
          {entry.riskLevel !== "safe" && (
            <RiskBadge risk={entry.riskLevel} reasons={entry.riskReasons} />
          )}
          {entry.tags.map((tag) => (
            <span key={tag} className="meta-tag">
              #{tag}
            </span>
          ))}
          {entry.collections.map((collection) => (
            <span key={collection.id} className="meta-pill meta-pill--collection">
              {collection.name}
            </span>
          ))}
        </div>

        <pre className="code entry-dialog__code" tabIndex={0}>
          <code>{resolved}</code>
        </pre>

        {entry.description && <p className="card__description">{entry.description}</p>}

        {entry.riskReasons.length > 0 && (
          <div className="card__warnings" data-risk={entry.riskLevel} role="note">
            <strong>Before you run this</strong>
            <ul>
              {entry.riskReasons.map((reason) => (
                <li key={reason}>{reason}</li>
              ))}
            </ul>
          </div>
        )}

        {entry.variables.length > 0 && (
          <div className="card__variables">
            <p className="card__section-label">Fill the placeholders, then copy</p>
            <div className="variable-grid">
              {entry.variables.map((variable) => (
                <label key={variable} className="variable">
                  <span>{variable}</span>
                  <input
                    className="input input--compact"
                    value={values[variable] ?? ""}
                    onChange={(event) =>
                      setValues((current) => ({ ...current, [variable]: event.target.value }))
                    }
                    autoComplete="off"
                    spellCheck={false}
                  />
                </label>
              ))}
            </div>
          </div>
        )}

        {entry.notes && (
          <div className="card__notes">
            <p className="card__section-label">Notes</p>
            <p>{entry.notes}</p>
          </div>
        )}

        <dl className="card__facts">
          {entry.operatingSystem && (
            <div>
              <dt>OS</dt>
              <dd>{entry.operatingSystem}</dd>
            </div>
          )}
          {entry.workingDirectory && (
            <div>
              <dt>Run from</dt>
              <dd className="mono">{entry.workingDirectory}</dd>
            </div>
          )}
          {entry.language && (
            <div>
              <dt>Language</dt>
              <dd>{entry.language}</dd>
            </div>
          )}
          <div>
            <dt>Updated</dt>
            <dd>{relativeTime(entry.updatedAt)}</dd>
          </div>
        </dl>

        {entry.sourceUrl && (
          <Button variant="ghost" size="sm" onClick={() => onOpenSource(entry.sourceUrl as string)}>
            <ExternalLink size={14} aria-hidden="true" />
            Open source
          </Button>
        )}
      </div>
    </Modal>
  );
}
