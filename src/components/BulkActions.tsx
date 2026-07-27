import { useState } from "react";
import { FolderPlus, ListChecks, Trash2, X } from "lucide-react";

import { api, toAppError } from "../lib/ipc";
import type { Collection } from "../lib/types";
import { ConfirmDialog } from "./ConfirmDialog";
import { Button } from "./ui/Button";
import { SelectField } from "./ui/Field";
import { Modal } from "./ui/Modal";
import { useToast } from "./ui/Toast";

export interface BulkActionsProps {
  collections: Collection[];
  selecting: boolean;
  selectedIds: ReadonlySet<number>;
  visibleCount: number;
  allVisibleSelected: boolean;
  onStart: () => void;
  onToggleAll: () => void;
  onCancel: () => void;
  onComplete: () => Promise<void>;
}

export function BulkActions({
  collections,
  selecting,
  selectedIds,
  visibleCount,
  allVisibleSelected,
  onStart,
  onToggleAll,
  onCancel,
  onComplete,
}: BulkActionsProps) {
  const [collectionOpen, setCollectionOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [collectionId, setCollectionId] = useState("");
  const [busy, setBusy] = useState(false);
  const { notify } = useToast();
  const selectedCount = selectedIds.size;

  if (!selecting) {
    return (
      <Button variant="secondary" size="sm" disabled={visibleCount === 0} onClick={onStart}>
        <ListChecks size={15} aria-hidden="true" />
        <span className="topbar__action-label">Select</span>
      </Button>
    );
  }

  const addToCollection = async () => {
    const target = Number(collectionId);
    if (!Number.isInteger(target) || target <= 0 || selectedCount === 0) return;

    setBusy(true);
    try {
      await api.addCommandsToCollection([...selectedIds], target);
      notify(
        `Added ${selectedCount} ${selectedCount === 1 ? "entry" : "entries"} to the collection`,
        "success",
      );
      setCollectionOpen(false);
      setCollectionId("");
      await onComplete();
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    } finally {
      setBusy(false);
    }
  };

  const removeSelected = async () => {
    if (selectedCount === 0) return;

    setBusy(true);
    try {
      await api.deleteCommands([...selectedIds]);
      notify(
        `Deleted ${selectedCount} ${selectedCount === 1 ? "entry" : "entries"}`,
        "success",
      );
      setDeleteOpen(false);
      await onComplete();
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="bulk-actions" role="toolbar" aria-label="Bulk command actions">
        <span className="bulk-actions__count" aria-live="polite">
          {selectedCount} <span className="bulk-actions__count-label">selected</span>
        </span>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label={allVisibleSelected ? "Clear selection" : "Select all visible entries"}
          title={allVisibleSelected ? "Clear selection" : "Select all visible entries"}
          onClick={onToggleAll}
        >
          <ListChecks size={16} aria-hidden="true" />
        </Button>
        <Button
          variant="secondary"
          size="sm"
          iconOnly
          disabled={selectedCount === 0 || collections.length === 0}
          aria-label="Add selected entries to a collection"
          title="Add to collection"
          onClick={() => setCollectionOpen(true)}
        >
          <FolderPlus size={16} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          className="bulk-actions__delete"
          disabled={selectedCount === 0}
          aria-label="Delete selected entries"
          title="Delete selected"
          onClick={() => setDeleteOpen(true)}
        >
          <Trash2 size={16} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label="Finish selecting"
          title="Finish selecting"
          onClick={onCancel}
        >
          <X size={16} aria-hidden="true" />
        </Button>
      </div>

      <Modal
        open={collectionOpen}
        title="Add to collection"
        description={`Existing collection memberships will stay in place for the ${selectedCount} selected ${
          selectedCount === 1 ? "entry" : "entries"
        }.`}
        onClose={() => {
          if (!busy) setCollectionOpen(false);
        }}
        footer={
          <>
            <Button variant="ghost" disabled={busy} onClick={() => setCollectionOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              loading={busy}
              disabled={!collectionId}
              onClick={() => void addToCollection()}
            >
              Add to collection
            </Button>
          </>
        }
      >
        <SelectField
          label="Collection"
          value={collectionId}
          onChange={(event) => setCollectionId(event.target.value)}
          options={[
            { value: "", label: "Choose a collection" },
            ...collections.map((collection) => ({
              value: String(collection.id),
              label: collection.name,
            })),
          ]}
        />
      </Modal>

      <ConfirmDialog
        open={deleteOpen}
        title={`Delete ${selectedCount} selected ${
          selectedCount === 1 ? "entry" : "entries"
        }?`}
        body="The selected entries will be removed from the library. This cannot be undone."
        confirmLabel="Delete selected"
        busy={busy}
        onConfirm={() => void removeSelected()}
        onCancel={() => {
          if (!busy) setDeleteOpen(false);
        }}
      />
    </>
  );
}
