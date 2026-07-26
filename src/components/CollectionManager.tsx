import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { Check, Pencil, Trash2, X } from "lucide-react";

import { api, toAppError } from "../lib/ipc";
import type { Collection } from "../lib/types";
import { Button } from "./ui/Button";
import { TextField } from "./ui/Field";
import { Modal } from "./ui/Modal";

export interface CollectionManagerProps {
  open: boolean;
  intent: CollectionManagerIntent;
  collections: Collection[];
  onClose: () => void;
  onChanged: () => void;
  onRequestDelete: (collection: Collection) => void;
}

export type CollectionManagerIntent =
  | { type: "manage" }
  | { type: "create" }
  | { type: "rename"; collectionId: number };

/** Create, rename, and remove collections. Entries always survive a removal. */
export function CollectionManager({
  open,
  intent,
  collections,
  onClose,
  onChanged,
  onRequestDelete,
}: CollectionManagerProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingName, setEditingName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;

    setError(null);
    if (intent.type === "create") {
      setName("");
      setDescription("");
      setEditingId(null);
      return;
    }

    if (intent.type === "rename") {
      const collection = collections.find((candidate) => candidate.id === intent.collectionId);
      setEditingId(collection?.id ?? null);
      setEditingName(collection?.name ?? "");
      return;
    }

    setEditingId(null);
    // Apply the opening intent once. A collection refresh after save should not
    // put the manager back into rename mode.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, intent]);

  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
      onChanged();
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  };

  const create = async (event: FormEvent) => {
    event.preventDefault();
    if (name.trim().length === 0) {
      setError("A collection needs a name");
      return;
    }
    await run(async () => {
      await api.createCollection({ name, description });
      setName("");
      setDescription("");
    });
  };

  const rename = async (collection: Collection) => {
    await run(async () => {
      await api.updateCollection(collection.id, {
        name: editingName,
        description: collection.description,
      });
      setEditingId(null);
    });
  };

  return (
    <Modal
      open={open}
      title="Collections"
      description="Collections say why commands belong together. Tags say what they are about."
      onClose={onClose}
      footer={
        <Button variant="secondary" onClick={onClose}>
          Done
        </Button>
      }
    >
      {error && (
        <p className="form__error" role="alert">
          {error}
        </p>
      )}

      <form className="form form--inline" onSubmit={create}>
        <TextField
          label="New collection"
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="Utilities"
          autoFocus={intent.type === "create"}
          data-modal-autofocus={intent.type === "create" ? "" : undefined}
        />
        <TextField
          label="Description"
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          placeholder="Optional"
        />
        <Button variant="primary" type="submit" loading={busy}>
          Add
        </Button>
      </form>

      {collections.length === 0 ? (
        <p className="field__hint">No collections yet.</p>
      ) : (
        <ul className="manager-list">
          {collections.map((collection) => (
            <li key={collection.id} className="manager-row">
              {editingId === collection.id ? (
                <>
                  <input
                    className="input input--compact"
                    value={editingName}
                    onChange={(event) => setEditingName(event.target.value)}
                    aria-label={`Rename ${collection.name}`}
                    autoFocus
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label="Save name"
                    onClick={() => void rename(collection)}
                  >
                    <Check size={15} aria-hidden="true" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label="Cancel rename"
                    onClick={() => setEditingId(null)}
                  >
                    <X size={15} aria-hidden="true" />
                  </Button>
                </>
              ) : (
                <>
                  <div className="manager-row__text">
                    <span className="manager-row__name">{collection.name}</span>
                    <span className="manager-row__meta">
                      {collection.commandCount}
                      {collection.commandCount === 1 ? " command" : " commands"}
                      {collection.description ? ` · ${collection.description}` : ""}
                    </span>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label={`Rename ${collection.name}`}
                    onClick={() => {
                      setEditingId(collection.id);
                      setEditingName(collection.name);
                    }}
                  >
                    <Pencil size={15} aria-hidden="true" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label={`Delete ${collection.name}`}
                    onClick={() => onRequestDelete(collection)}
                  >
                    <Trash2 size={15} aria-hidden="true" />
                  </Button>
                </>
              )}
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
