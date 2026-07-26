import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { CollectionActions } from "./CollectionActions";

function renderActions(callbacks?: {
  onRename?: () => void;
  onDelete?: () => void;
  onManageAll?: () => void;
}) {
  return render(
    <>
      <CollectionActions
        collectionName="Docker"
        onRename={callbacks?.onRename ?? vi.fn()}
        onDelete={callbacks?.onDelete ?? vi.fn()}
        onManageAll={callbacks?.onManageAll ?? vi.fn()}
      />
      <button type="button">Outside action</button>
    </>,
  );
}

describe("CollectionActions", () => {
  it("exposes a labelled disclosure without partial menu semantics", async () => {
    const user = userEvent.setup();
    renderActions();

    const trigger = screen.getByRole("button", { name: "Collection options for Docker" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    await user.click(trigger);

    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: "Rename collection" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete collection" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Manage all collections" })).toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.queryByRole("menuitem")).not.toBeInTheDocument();
  });

  it("closes on Escape and restores focus to the trigger", async () => {
    const user = userEvent.setup();
    renderActions();

    const trigger = screen.getByRole("button", { name: "Collection options for Docker" });
    await user.click(trigger);
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("button", { name: "Rename collection" })).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("closes when focus moves to an outside pointer target", async () => {
    const user = userEvent.setup();
    renderActions();

    await user.click(screen.getByRole("button", { name: "Collection options for Docker" }));
    await user.click(screen.getByRole("button", { name: "Outside action" }));

    expect(screen.queryByRole("button", { name: "Rename collection" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Outside action" })).toHaveFocus();
  });

  it.each([
    ["Rename collection", "onRename"],
    ["Delete collection", "onDelete"],
    ["Manage all collections", "onManageAll"],
  ] as const)("runs %s and closes the disclosure", async (label, callbackName) => {
    const user = userEvent.setup();
    const callbacks = {
      onRename: vi.fn(),
      onDelete: vi.fn(),
      onManageAll: vi.fn(),
    };
    renderActions(callbacks);

    await user.click(screen.getByRole("button", { name: "Collection options for Docker" }));
    await user.click(screen.getByRole("button", { name: label }));

    expect(callbacks[callbackName]).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: "Rename collection" })).not.toBeInTheDocument();
  });
});
