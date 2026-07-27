import { Copy, Maximize2, Pencil, Star, Trash2 } from "lucide-react";

import { kindLabel, lineCount } from "../lib/format";
import type { CommandEntry, CommandViewMode } from "../lib/types";
import { Button } from "./ui/Button";
import { RiskBadge } from "./ui/RiskBadge";

export interface CommandCardProps {
  entry: CommandEntry;
  viewMode: CommandViewMode;
  selecting?: boolean;
  selected?: boolean;
  onToggleSelection?: () => void;
  onOpenDetails: () => void;
  onCopy: (text: string) => void;
  onEdit: () => void;
  onDelete: () => void;
  onToggleFavorite: () => void;
}

export function CommandCard({
  entry,
  viewMode,
  selecting = false,
  selected = false,
  onToggleSelection,
  onOpenDetails,
  onCopy,
  onEdit,
  onDelete,
  onToggleFavorite,
}: CommandCardProps) {
  const lines = lineCount(entry.content);
  const openOrSelect = selecting ? onToggleSelection : onOpenDetails;

  return (
    <article
      className={`card card--${viewMode}${selecting ? " is-selecting" : ""}${
        selected ? " is-selected" : ""
      }`}
      data-risk={entry.riskLevel}
    >
      <div className="card__head">
        {selecting && (
          <label className="card__selection">
            <input
              type="checkbox"
              checked={selected}
              onChange={onToggleSelection}
              aria-label={`Select ${entry.title}`}
            />
          </label>
        )}
        <button
          type="button"
          className="card__toggle"
          onClick={openOrSelect}
          aria-haspopup={selecting ? undefined : "dialog"}
          aria-pressed={selecting ? selected : undefined}
          aria-label={selecting ? `Toggle selection for ${entry.title}` : undefined}
          data-card-focus="true"
        >
          {!selecting && <Maximize2 size={14} aria-hidden="true" className="card__open-icon" />}
          <span className="card__title">{entry.title}</span>
        </button>
      </div>

      <div className="card__status">
        {entry.riskLevel !== "safe" && (
          <RiskBadge iconOnly risk={entry.riskLevel} reasons={entry.riskReasons} />
        )}
      </div>

      <button
        type="button"
        className="code card__preview"
        onClick={openOrSelect}
        aria-label={
          selecting
            ? `Toggle selection for ${entry.title} from its preview`
            : `View full content for ${entry.title}`
        }
        aria-haspopup={selecting ? undefined : "dialog"}
        aria-pressed={selecting ? selected : undefined}
        title={selecting ? "Toggle selection" : "View full entry"}
      >
        <code>{entry.content}</code>
      </button>

      <div className="card__meta">
        {!selecting && (
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
        )}
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
      </div>

      {!selecting && <div className="card__actions">
        <Button
          variant="secondary"
          size="sm"
          iconOnly
          aria-label={`Copy ${entry.title}`}
          title="Copy"
          onClick={() => onCopy(entry.content)}
        >
          <Copy size={14} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label={`Edit ${entry.title}`}
          title="Edit"
          onClick={onEdit}
        >
          <Pencil size={14} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          className="card__delete"
          onClick={onDelete}
          aria-label={`Delete ${entry.title}`}
          title="Delete"
        >
          <Trash2 size={14} aria-hidden="true" />
        </Button>
      </div>}

    </article>
  );
}
