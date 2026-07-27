import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { makeEntry } from "../test/factories";
import { useBulkSelection } from "./useBulkSelection";

describe("useBulkSelection", () => {
  it("selects only visible entries and can clear them together", () => {
    const entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
    const { result } = renderHook(() => useBulkSelection(entries, "all"));

    act(() => result.current.start());
    act(() => result.current.toggleAllVisible());

    expect(result.current.selecting).toBe(true);
    expect([...result.current.selectedIds]).toEqual([1, 2]);
    expect(result.current.allVisibleSelected).toBe(true);

    act(() => result.current.toggleAllVisible());
    expect(result.current.selectedCount).toBe(0);
  });

  it("clears selection when the library context changes", () => {
    const entries = [makeEntry({ id: 1 })];
    const { result, rerender } = renderHook(
      ({ resetKey }) => useBulkSelection(entries, resetKey),
      { initialProps: { resetKey: "all" } },
    );

    act(() => result.current.start());
    act(() => result.current.toggle(1));
    expect(result.current.selectedCount).toBe(1);

    rerender({ resetKey: "favorites" });
    expect(result.current.selecting).toBe(false);
    expect(result.current.selectedCount).toBe(0);
  });
});
