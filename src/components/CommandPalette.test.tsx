import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Plus, Settings } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import type { PaletteAction } from "./CommandPalette";
import { CommandPalette } from "./CommandPalette";

function actions(): PaletteAction[] {
  return [
    {
      id: "add",
      label: "Add command",
      description: "Create a new entry",
      icon: Plus,
      keywords: ["new"],
      onSelect: vi.fn(),
    },
    {
      id: "settings",
      label: "Open Settings",
      description: "Change app behavior",
      icon: Settings,
      onSelect: vi.fn(),
    },
  ];
}

describe("CommandPalette", () => {
  it("filters actions and runs the selected result", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const available = actions();

    render(<CommandPalette open actions={available} onClose={onClose} />);

    const input = screen.getByRole("searchbox", { name: "Find an action" });
    expect(input).toHaveFocus();

    await user.type(input, "new");
    expect(screen.getByRole("button", { name: /Add command/ })).toBeVisible();
    expect(screen.queryByRole("button", { name: /Open Settings/ })).not.toBeInTheDocument();

    await user.keyboard("{Enter}");
    expect(onClose).toHaveBeenCalledOnce();
    expect(available[0].onSelect).toHaveBeenCalledOnce();
  });

  it("supports arrow-key selection and an honest empty result", async () => {
    const user = userEvent.setup();
    const available = actions();

    render(<CommandPalette open actions={available} onClose={vi.fn()} />);

    const input = screen.getByRole("searchbox", { name: "Find an action" });
    await user.keyboard("{ArrowDown}{Enter}");
    expect(available[1].onSelect).toHaveBeenCalledOnce();

    await user.clear(input);
    await user.type(input, "definitely missing");
    expect(screen.getByRole("status")).toHaveTextContent("No matching actions");
  });
});
