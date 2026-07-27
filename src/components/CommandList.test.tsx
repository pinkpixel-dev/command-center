import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { makeEntry } from "../test/factories";
import { CommandList } from "./CommandList";

function setup(overrides: Partial<Parameters<typeof CommandList>[0]> = {}) {
  const entries = [
    makeEntry(),
    makeEntry({
      id: 2,
      title: "Find a listening process",
      content: "lsof -i :3000",
      riskLevel: "safe",
      riskReasons: [],
    }),
  ];
  const handlers = {
    onOpenEntry: vi.fn(),
    onCopy: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onToggleFavorite: vi.fn(),
    onOpenSource: vi.fn(),
    onAskAssistant: vi.fn(),
    onReviewProposal: vi.fn(),
    onAdd: vi.fn(),
    onRetry: vi.fn(),
  };

  const view = render(
    <CommandList
      entries={entries}
      viewMode="compact"
      aiReady={false}
      loading={false}
      error={null}
      searching={false}
      openEntryId={null}
      {...handlers}
      {...overrides}
    />,
  );

  return { ...handlers, ...view };
}

describe("CommandList", () => {
  it("applies compact presentation to the list and each entry", () => {
    const { container } = setup();

    expect(container.querySelector(".command-list")).toHaveClass("command-list--compact");
    expect(container.querySelectorAll(".card--compact")).toHaveLength(2);
  });

  it("opens details without changing the surrounding card grid", () => {
    const { container } = setup({ viewMode: "cards", openEntryId: 2 });

    expect(container.querySelector(".command-list")).toHaveClass("command-list--cards");
    expect(container.querySelectorAll(".card--cards")).toHaveLength(2);
    expect(container.querySelectorAll(".command-list__item")[1]).not.toHaveClass("is-expanded");
    expect(screen.getByRole("dialog", { name: "Find a listening process" })).toBeInTheDocument();
  });

  it("keeps arrow-key navigation across command toggles", async () => {
    const user = userEvent.setup();
    setup();

    const first = screen.getByRole("button", { name: "Update Arch packages" });
    const second = screen.getByRole("button", { name: "Find a listening process" });

    first.focus();
    await user.keyboard("{ArrowDown}");
    expect(second).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    expect(first).toHaveFocus();

    await user.keyboard("{ArrowUp}");
    expect(second).toHaveFocus();
  });

  it.each(["compact", "cards"] as const)(
    "supports explicit selection in %s view without opening entries",
    async (viewMode) => {
      const user = userEvent.setup();
      const onToggleSelection = vi.fn();
      const onOpenEntry = vi.fn();
      const { container } = setup({
        viewMode,
        selecting: true,
        selectedIds: new Set([1]),
        onToggleSelection,
        onOpenEntry,
      });

      expect(screen.getByRole("checkbox", { name: "Select Update Arch packages" })).toBeChecked();
      expect(container.querySelectorAll(".is-selecting")).toHaveLength(2);
      expect(container.querySelectorAll(".is-selected")).toHaveLength(1);
      expect(screen.queryByRole("button", { name: "Copy Update Arch packages" })).not.toBeInTheDocument();

      await user.click(
        screen.getByRole("button", { name: "Toggle selection for Find a listening process" }),
      );
      expect(onToggleSelection).toHaveBeenCalledWith(2);
      expect(onOpenEntry).not.toHaveBeenCalled();
    },
  );
});
