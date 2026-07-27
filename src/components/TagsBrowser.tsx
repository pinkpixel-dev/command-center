import { Tags } from "lucide-react";

import type { Tag } from "../lib/types";
import { EmptyState } from "./ui/EmptyState";

export interface TagsBrowserProps {
  tags: Tag[];
  activeTagName: string | null;
  onSelectTag: (tagName: string) => void;
}

/** A read-only tag browser. Tags remain derived from saved command metadata. */
export function TagsBrowser({ tags, activeTagName, onSelectTag }: TagsBrowserProps) {
  return (
    <section className="organization-browser" aria-label="All tags">
      <header className="organization-browser__header">
        <div>
          <h2>Tags</h2>
          <p>Browse the tools and topics attached to your saved commands.</p>
        </div>
        <span className="organization-browser__total">
          {tags.length} {tags.length === 1 ? "tag" : "tags"}
        </span>
      </header>

      {tags.length === 0 ? (
        <EmptyState
          icon={Tags}
          title="No tags yet"
          body="Tags appear here as you add them to commands."
        />
      ) : (
        <ul className="organization-browser__tag-list">
          {tags.map((tag) => {
            const active = tag.name === activeTagName;
            return (
              <li key={tag.id}>
                <button
                  type="button"
                  className={`organization-browser__tag${active ? " is-active" : ""}`}
                  aria-pressed={active}
                  onClick={() => onSelectTag(tag.name)}
                >
                  <span>{tag.name}</span>
                  <span
                    className="organization-browser__count"
                    aria-label={`${tag.commandCount} ${
                      tag.commandCount === 1 ? "command" : "commands"
                    }`}
                  >
                    {tag.commandCount}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
