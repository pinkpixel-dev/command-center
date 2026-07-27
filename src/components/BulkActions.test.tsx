import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import { BulkActions } from "./BulkActions";
import { ToastProvider } from "./ui/Toast";

vi.mock("../lib/ipc", async () => {
  const actual = await vi.importActual<typeof import("../lib/ipc")>("../lib/ipc");
  return {
    ...actual,
    api: {
      ...actual.api,
      addCommandsToCollection: vi.fn(),
      deleteCommands: vi.fn(),
    },
  };
});

const collection = {
  id: 7,
  name: "Utilities",
  description: "",
  commandCount: 2,
  createdAt: "2026-07-01T00:00:00Z",
  updatedAt: "2026-07-01T00:00:00Z",
};

function setup(selectedIds: ReadonlySet<number> = new Set([1, 2])) {
  const onComplete = vi.fn().mockResolvedValue(undefined);
  render(
    <ToastProvider>
      <BulkActions
        collections={[collection]}
        selecting
        selectedIds={selectedIds}
        visibleCount={2}
        allVisibleSelected
        onStart={vi.fn()}
        onToggleAll={vi.fn()}
        onCancel={vi.fn()}
        onComplete={onComplete}
      />
    </ToastProvider>,
  );
  return { onComplete };
}

describe("BulkActions", () => {
  it("adds selected entries without replacing existing memberships", async () => {
    const user = userEvent.setup();
    vi.mocked(api.addCommandsToCollection).mockResolvedValue(undefined);
    const { onComplete } = setup();

    await user.click(screen.getByRole("button", { name: "Add selected entries to a collection" }));
    expect(
      screen.getByText(/Existing collection memberships will stay in place/),
    ).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText("Collection"), "7");
    await user.click(screen.getByRole("button", { name: "Add to collection" }));

    expect(api.addCommandsToCollection).toHaveBeenCalledWith([1, 2], 7);
    expect(onComplete).toHaveBeenCalledOnce();
  });

  it("requires confirmation before deleting selected entries", async () => {
    const user = userEvent.setup();
    vi.mocked(api.deleteCommands).mockResolvedValue(undefined);
    const { onComplete } = setup();

    await user.click(screen.getByRole("button", { name: "Delete selected entries" }));
    expect(api.deleteCommands).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Delete selected" }));

    expect(api.deleteCommands).toHaveBeenCalledWith([1, 2]);
    expect(onComplete).toHaveBeenCalledOnce();
  });
});
