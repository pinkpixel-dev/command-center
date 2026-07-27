import { FolderOpen } from "lucide-react";

import type { Collection } from "../lib/types";
import { Button } from "./ui/Button";
import { EmptyState } from "./ui/EmptyState";

export interface CollectionsBrowserProps {
  collections: Collection[];
  activeCollectionId: number | null;
  onSelectCollection: (collectionId: number) => void;
}

/** A read-only collection browser. Editing stays in the existing collection manager. */
export function CollectionsBrowser({
  collections,
  activeCollectionId,
  onSelectCollection,
}: CollectionsBrowserProps) {
  return (
    <section className="organization-browser" aria-label="All collections">
      <header className="organization-browser__header">
        <div>
          <h2>Collections</h2>
          <p>Browse the workflows and projects that organize your command library.</p>
        </div>
        <span className="organization-browser__total">
          {collections.length} {collections.length === 1 ? "collection" : "collections"}
        </span>
      </header>

      {collections.length === 0 ? (
        <EmptyState
          icon={FolderOpen}
          title="No collections yet"
          body="Create a collection to keep related commands together."
        />
      ) : (
        <ul className="organization-browser__list">
          {collections.map((collection) => {
            const active = collection.id === activeCollectionId;
            return (
              <li
                key={collection.id}
                className={`organization-browser__row${active ? " is-active" : ""}`}
              >
                <div className="organization-browser__details">
                  <h3>{collection.name}</h3>
                  {collection.description && <p>{collection.description}</p>}
                </div>
                <span
                  className="organization-browser__count"
                  aria-label={`${collection.commandCount} ${
                    collection.commandCount === 1 ? "command" : "commands"
                  }`}
                >
                  {collection.commandCount}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={`Open ${collection.name} collection`}
                  aria-current={active ? "page" : undefined}
                  onClick={() => onSelectCollection(collection.id)}
                >
                  Open
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
