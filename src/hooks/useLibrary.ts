import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, LIBRARY_CHANGED, toAppError } from "../lib/ipc";
import { platform } from "../lib/platform";
import type { Collection, CommandEntry, LibraryStats, ListQuery, Tag } from "../lib/types";

const EMPTY_STATS: LibraryStats = { total: 0, favorites: 0, scripts: 0, recent: 0 };

export interface LibraryData {
  entries: CommandEntry[];
  stats: LibraryStats;
  tags: Tag[];
  collections: Collection[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

/**
 * Loads the slice of the library described by `filter`, plus the sidebar data,
 * and keeps itself current when any window changes something.
 */
export function useLibrary(filter: ListQuery): LibraryData {
  const [entries, setEntries] = useState<CommandEntry[]>([]);
  const [stats, setStats] = useState<LibraryStats>(EMPTY_STATS);
  const [tags, setTags] = useState<Tag[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Serialising the filter keeps the effect from re-running on every render
  // just because the caller built a fresh object literal.
  const filterKey = JSON.stringify(filter);
  const filterRef = useRef(filter);
  filterRef.current = filter;

  const load = useCallback(async () => {
    try {
      const [nextEntries, nextStats, nextTags, nextCollections] = await Promise.all([
        api.listCommands(filterRef.current),
        api.libraryStats(),
        api.listTags(),
        api.listCollections(),
      ]);
      setEntries(nextEntries);
      setStats(nextStats);
      setTags(nextTags);
      setCollections(nextCollections);
      setError(null);
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void load().then(() => {
      if (!active) return;
    });
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterKey, load]);

  // Follow writes announced by the backend, including dormant import work.
  useEffect(() => {
    return platform.subscribe(LIBRARY_CHANGED, () => {
      void load();
    });
  }, [load]);

  return useMemo(
    () => ({ entries, stats, tags, collections, loading, error, refresh: load }),
    [entries, stats, tags, collections, loading, error, load],
  );
}
