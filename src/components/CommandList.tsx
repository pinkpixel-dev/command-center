import { useRef } from "react";
import type { KeyboardEvent } from "react";
import { Inbox, SearchX } from "lucide-react";

import type { CommandEntry, CommandViewMode } from "../lib/types";
import { CommandCard } from "./CommandCard";
import { CommandDetailsDialog } from "./CommandDetailsDialog";
import { Button } from "./ui/Button";
import { EmptyState } from "./ui/EmptyState";

export interface CommandListProps {
  entries: CommandEntry[];
  viewMode: CommandViewMode;
  loading: boolean;
  error: string | null;
  searching: boolean;
  openEntryId: number | null;
  onOpenEntry: (id: number | null) => void;
  onCopy: (entry: CommandEntry, text: string) => void;
  onEdit: (entry: CommandEntry) => void;
  onDelete: (entry: CommandEntry) => void;
  onToggleFavorite: (entry: CommandEntry) => void;
  onOpenSource: (url: string) => void;
  onAdd: () => void;
  onRetry: () => void;
}

export function CommandList({
  entries,
  viewMode,
  loading,
  error,
  searching,
  openEntryId,
  onOpenEntry,
  onCopy,
  onEdit,
  onDelete,
  onToggleFavorite,
  onOpenSource,
  onAdd,
  onRetry,
}: CommandListProps) {
  const listRef = useRef<HTMLUListElement>(null);

  // Up and down walk the list without leaving the keyboard.
  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;

    const focusables = Array.from(
      listRef.current?.querySelectorAll<HTMLElement>("[data-card-focus]") ?? [],
    );
    if (focusables.length === 0) return;

    const current = focusables.indexOf(document.activeElement as HTMLElement);
    if (current === -1) return;

    event.preventDefault();
    const step = event.key === "ArrowDown" ? 1 : -1;
    const next = (current + step + focusables.length) % focusables.length;
    focusables[next].focus();
  };

  if (error) {
    return (
      <div className="list-state" role="alert">
        <EmptyState
          icon={SearchX}
          title="The library could not be read"
          body={error}
          action={
            <Button variant="secondary" onClick={onRetry}>
              Try again
            </Button>
          }
        />
      </div>
    );
  }

  if (loading) {
    return (
      <div className="list-state" aria-busy="true" aria-live="polite">
        <span className="visually-hidden">Loading commands</span>
        {[0, 1, 2].map((index) => (
          <div key={index} className="skeleton-card" aria-hidden="true">
            <div className="skeleton skeleton--title" />
            <div className="skeleton skeleton--code" />
            <div className="skeleton skeleton--meta" />
          </div>
        ))}
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="list-state">
        {searching ? (
          <EmptyState
            icon={SearchX}
            title="Nothing matched that"
            body="Try fewer words, or search for part of the command itself."
          />
        ) : (
          <EmptyState
            icon={Inbox}
            title="No commands here yet"
            body="Save the next command you have to look up twice. It takes about ten seconds."
            action={
              <Button variant="primary" onClick={onAdd}>
                Add your first command
              </Button>
            }
          />
        )}
      </div>
    );
  }

  const openEntry = entries.find((entry) => entry.id === openEntryId) ?? null;

  return (
    <>
      <ul
        className={`command-list command-list--${viewMode}`}
        ref={listRef}
        onKeyDown={onKeyDown}
      >
        {entries.map((entry) => (
          <li key={entry.id} className="command-list__item">
            <CommandCard
              entry={entry}
              viewMode={viewMode}
              onOpenDetails={() => onOpenEntry(entry.id)}
              onCopy={(text) => onCopy(entry, text)}
              onEdit={() => onEdit(entry)}
              onDelete={() => onDelete(entry)}
              onToggleFavorite={() => onToggleFavorite(entry)}
            />
          </li>
        ))}
      </ul>

      {openEntry && (
        <CommandDetailsDialog
          entry={openEntry}
          open
          onClose={() => onOpenEntry(null)}
          onCopy={(text) => onCopy(openEntry, text)}
          onEdit={() => onEdit(openEntry)}
          onDelete={() => onDelete(openEntry)}
          onOpenSource={onOpenSource}
        />
      )}
    </>
  );
}
