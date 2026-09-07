import { platform } from "./platform";
import type { Collection } from "./types";

/**
 * The three ways a library leaves the app. Each one ends somewhere different
 * depending on where the app is running, so the work is the platform's: a save
 * dialog and a written file on the desktop, a download in a browser.
 *
 * Each resolves to where the file went, or null when the user backed out.
 */

export function exportLibraryMarkdown(): Promise<string | null> {
  return platform.exportLibraryMarkdown();
}

export function exportCollectionMarkdown(collection: Collection): Promise<string | null> {
  return platform.exportCollectionMarkdown(collection);
}

export function backupLibraryDatabase(): Promise<string | null> {
  return platform.backupLibrary();
}
