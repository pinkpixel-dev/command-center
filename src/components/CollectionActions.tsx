import { useEffect, useId, useRef, useState } from "react";
import { Download, Ellipsis, Pencil, Settings, Trash2 } from "lucide-react";

export interface CollectionActionsProps {
  collectionName: string;
  onRename: () => void;
  onDelete: () => void;
  onExport: () => void;
  onManageAll: () => void;
}

export function CollectionActions({
  collectionName,
  onRename,
  onDelete,
  onExport,
  onManageAll,
}: CollectionActionsProps) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const panelId = useId();

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && !containerRef.current?.contains(event.target)) {
        setOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setOpen(false);
      triggerRef.current?.focus();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  const runAction = (action: () => void) => {
    setOpen(false);
    action();
  };

  return (
    <div ref={containerRef} className="collection-actions">
      <button
        ref={triggerRef}
        type="button"
        className="button button--ghost button--sm button--icon"
        aria-label={`Collection options for ${collectionName}`}
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        title={`Collection options for ${collectionName}`}
        onClick={() => setOpen((current) => !current)}
      >
        <Ellipsis size={18} aria-hidden="true" />
      </button>

      {open && (
        <div
          id={panelId}
          className="collection-actions__panel"
          aria-label={`Options for ${collectionName}`}
        >
          <button type="button" onClick={() => runAction(onRename)}>
            <Pencil size={15} aria-hidden="true" />
            Rename collection
          </button>
          <button
            type="button"
            className="collection-actions__danger"
            onClick={() => runAction(onDelete)}
          >
            <Trash2 size={15} aria-hidden="true" />
            Delete collection
          </button>
          <button type="button" onClick={() => runAction(onExport)}>
            <Download size={15} aria-hidden="true" />
            Export collection
          </button>
          <span className="collection-actions__divider" aria-hidden="true" />
          <button type="button" onClick={() => runAction(onManageAll)}>
            <Settings size={15} aria-hidden="true" />
            Manage all collections
          </button>
        </div>
      )}
    </div>
  );
}
