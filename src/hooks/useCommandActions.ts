import { useCallback } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { copyToClipboard } from "../lib/clipboard";
import { api, toAppError } from "../lib/ipc";
import type { CommandEntry, CommandInput } from "../lib/types";
import { useToast } from "../components/ui/Toast";

export interface CommandActions {
  copy: (entry: CommandEntry, text: string) => Promise<void>;
  toggleFavorite: (entry: CommandEntry) => Promise<void>;
  remove: (entry: CommandEntry) => Promise<void>;
  openSource: (url: string) => Promise<void>;
  save: (input: CommandInput, id: number | null) => Promise<CommandEntry>;
}

/**
 * The write-side of the library. Every action reports what happened, and the
 * caller refreshes rather than guessing at optimistic state.
 */
export function useCommandActions(refresh: () => Promise<void>): CommandActions {
  const { notify } = useToast();

  const copy = useCallback(
    async (entry: CommandEntry, text: string) => {
      try {
        await copyToClipboard(text);
        await api.recordCopy(entry.id);
        notify(`Copied "${entry.title}"`, "success");
        await refresh();
      } catch (caught) {
        notify(toAppError(caught).message, "error");
      }
    },
    [notify, refresh],
  );

  const toggleFavorite = useCallback(
    async (entry: CommandEntry) => {
      try {
        const favorite = await api.toggleFavorite(entry.id);
        notify(favorite ? "Added to favorites" : "Removed from favorites", "info");
        await refresh();
      } catch (caught) {
        notify(toAppError(caught).message, "error");
      }
    },
    [notify, refresh],
  );

  const remove = useCallback(
    async (entry: CommandEntry) => {
      try {
        await api.deleteCommand(entry.id);
        notify(`Deleted "${entry.title}"`, "success");
        await refresh();
      } catch (caught) {
        notify(toAppError(caught).message, "error");
      }
    },
    [notify, refresh],
  );

  const openSource = useCallback(
    async (url: string) => {
      try {
        await openUrl(url);
      } catch (caught) {
        notify(toAppError(caught).message, "error");
      }
    },
    [notify],
  );

  const save = useCallback(
    async (input: CommandInput, id: number | null) => {
      const saved = id === null ? await api.createCommand(input) : await api.updateCommand(id, input);
      notify(id === null ? "Command saved" : "Changes saved", "success");
      await refresh();
      return saved;
    },
    [notify, refresh],
  );

  return { copy, toggleFavorite, remove, openSource, save };
}
