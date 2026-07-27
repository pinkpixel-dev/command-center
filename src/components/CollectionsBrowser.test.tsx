import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { Collection } from "../lib/types";
import { CollectionsBrowser } from "./CollectionsBrowser";

const collections: Collection[] = [
  {
    id: 1,
    name: "Daily tools",
    description: "Commands used during normal work",
    commandCount: 8,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  },
  {
    id: 2,
    name: "Release checks",
    description: "",
    commandCount: 1,
    createdAt: "2026-07-02T00:00:00Z",
    updatedAt: "2026-07-02T00:00:00Z",
  },
];

describe("CollectionsBrowser", () => {
  it("presents collection details, counts, and the active collection", async () => {
    const user = userEvent.setup();
    const onSelectCollection = vi.fn();

    render(
      <CollectionsBrowser
        collections={collections}
        activeCollectionId={2}
        onSelectCollection={onSelectCollection}
      />,
    );

    expect(screen.getByRole("heading", { name: "Collections" })).toBeVisible();
    expect(screen.getByText("Commands used during normal work")).toBeVisible();
    expect(screen.getByLabelText("8 commands")).toBeVisible();
    expect(screen.getByLabelText("1 command")).toBeVisible();

    const active = screen.getByRole("button", { name: "Open Release checks collection" });
    expect(active).toHaveAttribute("aria-current", "page");
    await user.click(active);
    expect(onSelectCollection).toHaveBeenCalledWith(2);
  });

  it("has a useful empty state", () => {
    render(
      <CollectionsBrowser
        collections={[]}
        activeCollectionId={null}
        onSelectCollection={vi.fn()}
      />,
    );

    expect(screen.getByRole("heading", { name: "No collections yet" })).toBeVisible();
    expect(screen.getByText(/keep related commands together/i)).toBeVisible();
  });
});
