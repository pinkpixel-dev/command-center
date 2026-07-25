import { useEffect, useRef } from "react";

import { isTypingTarget, matchesCombo } from "../lib/hotkeys";

export interface Hotkey {
  combo: string;
  handler: (event: KeyboardEvent) => void;
  /** Set when the shortcut should still fire while a field has focus. */
  allowWhileTyping?: boolean;
}

/** Binds a set of shortcuts to the document for as long as `enabled` is true. */
export function useHotkeys(hotkeys: Hotkey[], enabled = true): void {
  const ref = useRef(hotkeys);
  ref.current = hotkeys;

  useEffect(() => {
    if (!enabled) return;

    const onKeyDown = (event: KeyboardEvent) => {
      const typing = isTypingTarget(event.target);

      for (const hotkey of ref.current) {
        if (typing && !hotkey.allowWhileTyping) continue;
        if (!matchesCombo(hotkey.combo, event)) continue;
        event.preventDefault();
        hotkey.handler(event);
        return;
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [enabled]);
}
