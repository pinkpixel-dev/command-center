import { useMemo, useState } from "react";
import {
  ChevronDown,
  Copy,
  ExternalLink,
  Pencil,
  Star,
  Trash2,
} from "lucide-react";

import { copySummary, kindLabel, lineCount, relativeTime, renderTemplate } from "../lib/format";
import type { CommandEntry, CommandViewMode } from "../lib/types";
import { Button } from "./ui/Button";
import { RiskBadge } from "./ui/RiskBadge";

export interface CommandCardProps {
  entry: CommandEntry;
  viewMode: CommandViewMode;
  expanded: boolean;
  onToggle: () => void;
  onCopy: (text: string) => void;
  onEdit: () => void;
  onDelete: () => void;
  onToggleFavorite: () => void;
  onOpenSource: (url: string) => void;
}

export function CommandCard({
  entry,
  viewMode,
  expanded,
  onToggle,
  onCopy,
  onEdit,
  onDelete,
  onToggleFavorite,
  onOpenSource,
}: CommandCardProps) {
  const [values, setValues] = useState<Record<string, string>>({});

  const resolved = useMemo(
    () => (entry.variables.length > 0 ? renderTemplate(entry.content, values) : entry.content),
    [entry.content, entry.variables.length, values],
  );

  const lines = lineCount(entry.content);
  const bodyId = `command-body-${entry.id}`;

  return (
    <article
      className={`card card--${viewMode}${expanded ? " is-expanded" : ""}`}
      data-risk={entry.riskLevel}
    >
      <div className="card__head">
        <button
          type="button"
          className="card__toggle"
          onClick={onToggle}
          aria-expanded={expanded}
          aria-controls={bodyId}
          data-card-focus="true"
        >
          <ChevronDown size={15} aria-hidden="true" className="card__chevron" />
          <span className="card__title">{entry.title}</span>
        </button>

        <div className="card__head-meta">
          {entry.riskLevel !== "safe" && (
            <RiskBadge risk={entry.riskLevel} reasons={entry.riskReasons} />
          )}
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            className={entry.favorite ? "is-favorite" : ""}
            aria-pressed={entry.favorite}
            aria-label={entry.favorite ? "Remove from favorites" : "Add to favorites"}
            title={entry.favorite ? "Remove from favorites" : "Add to favorites"}
            onClick={onToggleFavorite}
          >
            <Star size={15} aria-hidden="true" fill={entry.favorite ? "currentColor" : "none"} />
          </Button>
        </div>
      </div>

      <pre className={`code${expanded ? "" : " code--clamped"}`}>
        <code>{expanded ? resolved : entry.content}</code>
      </pre>

      <div className="card__meta">
        <span className="meta-pill">{kindLabel(entry.kind)}</span>
        {entry.shell && <span className="meta-pill">{entry.shell}</span>}
        {lines > 1 && <span className="meta-pill">{lines} lines</span>}
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
        <span className="card__usage">
          {copySummary(entry)}
          {entry.lastCopiedAt ? ` · ${relativeTime(entry.lastCopiedAt)}` : ""}
        </span>
      </div>

      <div className="card__body" id={bodyId} hidden={!expanded}>
        {entry.description && <p className="card__description">{entry.description}</p>}

        {entry.riskReasons.length > 0 && (
          <div className="card__warnings" role="note">
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

      <div className="card__actions">
        <Button variant="secondary" size="sm" onClick={() => onCopy(resolved)}>
          <Copy size={14} aria-hidden="true" />
          Copy
        </Button>
        <Button variant="ghost" size="sm" onClick={onEdit}>
          <Pencil size={14} aria-hidden="true" />
          Edit
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="card__delete"
          onClick={onDelete}
          aria-label={`Delete ${entry.title}`}
        >
          <Trash2 size={14} aria-hidden="true" />
          <span className="card__delete-label">Delete</span>
        </Button>
      </div>
    </article>
  );
}
