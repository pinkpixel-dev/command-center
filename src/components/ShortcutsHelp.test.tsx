import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ShortcutsHelp } from "./ShortcutsHelp";

describe("ShortcutsHelp", () => {
  it("documents the AI assistant shortcut with both platform modifiers", () => {
    render(<ShortcutsHelp open onClose={vi.fn()} />);

    expect(screen.getByText("Open the AI assistant")).toBeVisible();
    expect(screen.getByText("Ctrl/Cmd")).toBeVisible();
    expect(screen.getByText("Shift")).toBeVisible();
    expect(screen.getAllByText("K")).toHaveLength(2);
  });
});
