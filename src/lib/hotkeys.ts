/** Keyboard matching kept separate from React so it can be tested directly. */

export interface KeyStroke {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/**
 * Matches a combo like `mod+k`, `shift+?`, `escape` or a bare `n`.
 * `mod` means Ctrl on Linux/Windows and Cmd on macOS.
 */
export function matchesCombo(combo: string, event: KeyStroke): boolean {
  const parts = combo.toLowerCase().split("+").map((part) => part.trim());
  const key = parts.pop() ?? "";

  const wantsMod = parts.includes("mod");
  const wantsShift = parts.includes("shift");
  const wantsAlt = parts.includes("alt");

  const hasMod = event.ctrlKey || event.metaKey;
  if (wantsMod !== hasMod) return false;
  if (wantsAlt !== event.altKey) return false;

  const eventKey = event.key.toLowerCase();

  // Shift is implied by symbols that need it (`?`), so only enforce it for
  // combos written with letters.
  if (wantsShift && !event.shiftKey) return false;
  if (!wantsShift && event.shiftKey && /^[a-z]$/.test(key)) return false;

  return eventKey === key;
}

/** True when the keystroke belongs to whatever the user is typing into. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select";
}
