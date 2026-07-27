import { useId, useMemo, useState } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";
import { X } from "lucide-react";

import { api, toAppError } from "../lib/ipc";
import type { Collection } from "../lib/types";
import { Button } from "./ui/Button";

const CREATE_COLLECTION = "__create_collection__";

export interface CollectionFieldProps {
  collections: Collection[];
  value: number[];
  onChange: (next: number[]) => void;
}

/** Multi-collection assignment with an inline path for creating a missing option. */
export function CollectionField({ collections, value, onChange }: CollectionFieldProps) {
  const [localCollections, setLocalCollections] = useState<Collection[]>([]);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selectId = useId();
  const nameId = useId();
  const errorId = `${nameId}-error`;

  const allCollections = useMemo(() => {
    const byId = new Map<number, Collection>();
    [...collections, ...localCollections].forEach((collection) => {
      byId.set(collection.id, collection);
    });
    return Array.from(byId.values()).sort((left, right) =>
      left.name.localeCompare(right.name, undefined, { sensitivity: "base" }),
    );
  }, [collections, localCollections]);

  const selected = value
    .map((id) => allCollections.find((collection) => collection.id === id))
    .filter((collection): collection is Collection => collection !== undefined);
  const available = allCollections.filter((collection) => !value.includes(collection.id));

  const choose = (event: ChangeEvent<HTMLSelectElement>) => {
    const selectedValue = event.target.value;
    if (selectedValue === CREATE_COLLECTION) {
      setCreating(true);
      setError(null);
      return;
    }
    if (!selectedValue) return;

    const id = Number(selectedValue);
    if (!value.includes(id)) {
      onChange([...value, id]);
    }
  };

  const cancelCreate = () => {
    setCreating(false);
    setName("");
    setError(null);
  };

  const create = async () => {
    const normalizedName = name.trim();
    if (!normalizedName) {
      setError("A collection needs a name");
      return;
    }

    setBusy(true);
    setError(null);
    try {
      const nextCollections = await api.createCollection({
        name: normalizedName,
        description: "",
      });
      const created = nextCollections.find(
        (collection) => collection.name.toLocaleLowerCase() === normalizedName.toLocaleLowerCase(),
      );
      if (!created) {
        setError("The collection was created, but it could not be selected. Reopen this form.");
        return;
      }

      setLocalCollections(nextCollections);
      onChange(value.includes(created.id) ? value : [...value, created.id]);
      cancelCreate();
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  };

  const handleNameKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      void create();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      cancelCreate();
    }
  };

  return (
    <div className="field collection-field">
      <label className="field__label" htmlFor={selectId}>
        Collections
      </label>

      {selected.length > 0 && (
        <ul className="collection-field__selected" aria-label="Selected collections">
          {selected.map((collection) => (
            <li key={collection.id} className="collection-field__chip">
              <span>{collection.name}</span>
              <button
                type="button"
                className="collection-field__remove"
                onClick={() => onChange(value.filter((id) => id !== collection.id))}
                aria-label={`Remove ${collection.name} collection`}
              >
                <X size={12} aria-hidden="true" />
              </button>
            </li>
          ))}
        </ul>
      )}

      <select
        id={selectId}
        className="input input--select"
        value=""
        onChange={choose}
        disabled={busy}
      >
        <option value="">Add to a collection…</option>
        {available.map((collection) => (
          <option key={collection.id} value={collection.id}>
            {collection.name}
          </option>
        ))}
        <option value={CREATE_COLLECTION}>Create new collection…</option>
      </select>

      {creating && (
        <div className="collection-field__create">
          <label className="field__label" htmlFor={nameId}>
            New collection name
          </label>
          <div className="collection-field__create-row">
            <input
              id={nameId}
              className="input"
              value={name}
              onChange={(event) => setName(event.target.value)}
              onKeyDown={handleNameKeyDown}
              placeholder="Utilities"
              disabled={busy}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? errorId : undefined}
              autoFocus
            />
            <Button variant="primary" size="sm" onClick={() => void create()} loading={busy}>
              Add
            </Button>
            <Button variant="ghost" size="sm" onClick={cancelCreate} disabled={busy}>
              Cancel
            </Button>
          </div>
          {error && (
            <p className="field__error" id={errorId} role="alert">
              <span aria-hidden="true">!</span> {error}
            </p>
          )}
        </div>
      )}

    </div>
  );
}
