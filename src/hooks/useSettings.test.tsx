import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ThemePreference } from "../lib/types";
import { useTheme } from "./useSettings";

afterEach(() => {
  delete document.documentElement.dataset.theme;
});

describe("useTheme", () => {
  it.each(["dark", "light", "high-contrast"] as const)(
    "applies the %s preference directly",
    (preference) => {
      renderHook(() => useTheme(preference as ThemePreference));

      expect(document.documentElement).toHaveAttribute("data-theme", preference);
    },
  );

  it("follows the system preference and removes its listener on unmount", () => {
    let prefersLight = true;
    let changeListener: (() => void) | undefined;
    const addEventListener = vi.fn((_event: string, listener: () => void) => {
      changeListener = listener;
    });
    const removeEventListener = vi.fn();

    vi.spyOn(window, "matchMedia").mockImplementation(
      (query) =>
        ({
          get matches() {
            return prefersLight;
          },
          media: query,
          onchange: null,
          addEventListener,
          removeEventListener,
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        }) as unknown as MediaQueryList,
    );

    const { unmount } = renderHook(() => useTheme("system"));

    expect(window.matchMedia).toHaveBeenCalledWith("(prefers-color-scheme: light)");
    expect(document.documentElement).toHaveAttribute("data-theme", "light");

    prefersLight = false;
    act(() => changeListener?.());
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");

    unmount();
    expect(removeEventListener).toHaveBeenCalledWith("change", changeListener);
  });
});
