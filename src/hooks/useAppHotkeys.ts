import type { RefObject } from "react";

import type { AppView } from "../lib/types";
import { useHotkeys } from "./useHotkeys";

export interface AppHotkeyOptions {
  view: AppView;
  aiReady: boolean;
  search: string;
  searchRef: RefObject<HTMLInputElement | null>;
  onCreate: () => void;
  onOpenPalette: () => void;
  onToggleAssistant: () => void;
  onOpenShortcuts: () => void;
  onClearSearch: () => void;
  onDismissNavigation: () => void;
}

export function useAppHotkeys({
  view,
  aiReady,
  search,
  searchRef,
  onCreate,
  onOpenPalette,
  onToggleAssistant,
  onOpenShortcuts,
  onClearSearch,
  onDismissNavigation,
}: AppHotkeyOptions): void {
  const libraryHotkeys = view === "library"
    ? [
        { combo: "/", handler: () => searchRef.current?.focus() },
        { combo: "n", handler: onCreate },
      ]
    : [];

  useHotkeys([
    ...libraryHotkeys,
    { combo: "mod+k", allowWhileTyping: true, handler: onOpenPalette },
    ...(aiReady
      ? [{
          combo: "mod+shift+k",
          allowWhileTyping: true,
          handler: onToggleAssistant,
        }]
      : []),
    { combo: "shift+?", handler: onOpenShortcuts },
    {
      combo: "escape",
      allowWhileTyping: true,
      handler: () => {
        if (document.activeElement === searchRef.current && search) {
          onClearSearch();
          return;
        }
        onDismissNavigation();
      },
    },
  ]);

}
