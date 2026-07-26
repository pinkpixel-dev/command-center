import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { Collection } from "../lib/types";
import { CollectionManager } from "./CollectionManager";

const collections: Collection[] = [
  {
    id: 7,
    name: "Utilities",
    description: "Handy everyday commands",
    commandCount: 3,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  },
];

function setup(
  intent: Parameters<typeof CollectionManager>[0]["intent"] = { type: "manage" },
) {
  const onClose = vi.fn();
  const onChanged = vi.fn();
  const onRequestDelete = vi.fn();

  render(
    <CollectionManager
      open
      intent={intent}
      collections={collections}
      onClose={onClose}
      onChanged={onChanged}
      onRequestDelete={onRequestDelete}
    />,
  );

  return { onClose, onChanged, onRequestDelete };
}

describe("CollectionManager", () => {
  it("opens directly in create mode and creates a collection", async () => {
    const user = userEvent.setup();
    const createCollection = vi
      .spyOn(api, "createCollection")
      .mockResolvedValue(collections);
    const { onChanged } = setup({ type: "create" });

    const name = screen.getByLabelText("New collection");
    expect(name).toHaveFocus();

    await user.type(name, "Git");
    await user.type(screen.getByLabelText("Description"), "Source control helpers");
    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(createCollection).toHaveBeenCalledWith({
      name: "Git",
      description: "Source control helpers",
    });
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("opens the requested collection directly in rename mode", async () => {
    const user = userEvent.setup();
    const updateCollection = vi
      .spyOn(api, "updateCollection")
      .mockResolvedValue(collections);
    const { onChanged } = setup({ type: "rename", collectionId: 7 });

    const name = screen.getByLabelText("Rename Utilities");
    expect(name).toHaveFocus();
    await user.clear(name);
    await user.type(name, "Everyday tools");
    await user.click(screen.getByRole("button", { name: "Save name" }));

    expect(updateCollection).toHaveBeenCalledWith(7, {
      name: "Everyday tools",
      description: "Handy everyday commands",
    });
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("requests confirmation instead of deleting immediately", async () => {
    const user = userEvent.setup();
    const deleteCollection = vi.spyOn(api, "deleteCollection");
    const { onRequestDelete } = setup();

    await user.click(screen.getByRole("button", { name: "Delete Utilities" }));

    expect(onRequestDelete).toHaveBeenCalledWith(collections[0]);
    expect(deleteCollection).not.toHaveBeenCalled();
  });
});
