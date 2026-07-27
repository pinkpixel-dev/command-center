import { useCallback } from "react";

import { useToast } from "../components/ui/Toast";
import { toAppError } from "../lib/ipc";
import {
  backupLibraryDatabase,
  exportCollectionMarkdown,
  exportLibraryMarkdown,
} from "../lib/library-files";
import type { Collection } from "../lib/types";

export function useLibraryFiles() {
  const { notify } = useToast();

  const exportLibrary = useCallback(async () => {
    try {
      const destination = await exportLibraryMarkdown();
      if (destination) notify("Markdown export saved", "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    }
  }, [notify]);

  const exportCollection = useCallback(async (collection: Collection) => {
    try {
      const destination = await exportCollectionMarkdown(collection);
      if (destination) notify(`${collection.name} export saved`, "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    }
  }, [notify]);

  const backupLibrary = useCallback(async () => {
    try {
      const destination = await backupLibraryDatabase();
      if (destination) notify("Library backup saved", "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    }
  }, [notify]);

  return { exportLibrary, exportCollection, backupLibrary };
}
