import { useCallback, useEffect, useMemo, useState } from "react";

import type { CommandEntry } from "../lib/types";

export interface BulkSelection {
  selecting: boolean;
  selectedIds: ReadonlySet<number>;
  selectedCount: number;
  allVisibleSelected: boolean;
  start: () => void;
  cancel: () => void;
  toggle: (id: number) => void;
  toggleAllVisible: () => void;
}

/** Keeps bulk selection local to the current, visible set of library results. */
export function useBulkSelection(
  entries: CommandEntry[],
  resetKey: string,
): BulkSelection {
  const [selecting, setSelecting] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(() => new Set());
  const visibleIds = useMemo(() => entries.map((entry) => entry.id), [entries]);
  const allVisibleSelected =
    visibleIds.length > 0 && visibleIds.every((id) => selectedIds.has(id));

  const cancel = useCallback(() => {
    setSelecting(false);
    setSelectedIds(new Set());
  }, []);

  useEffect(() => {
    cancel();
  }, [cancel, resetKey]);

  const toggle = useCallback((id: number) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const toggleAllVisible = useCallback(() => {
    setSelectedIds((current) => {
      const everyVisibleSelected =
        visibleIds.length > 0 && visibleIds.every((id) => current.has(id));
      if (everyVisibleSelected) return new Set();
      return new Set(visibleIds);
    });
  }, [visibleIds]);

  return {
    selecting,
    selectedIds,
    selectedCount: selectedIds.size,
    allVisibleSelected,
    start: () => setSelecting(true),
    cancel,
    toggle,
    toggleAllVisible,
  };
}
