import { createRef } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

describe("library navigation", () => {
  it("renders collection context actions beside Add command", () => {
    render(
      <TopBar
        title="All commands"
        subtitle="3 entries"
        search=""
        sort="updated"
        kind=""
        searchRef={createRef<HTMLInputElement>()}
        onSearchChange={vi.fn()}
        onSortChange={vi.fn()}
        onKindChange={vi.fn()}
        onAdd={vi.fn()}
        onOpenMenu={vi.fn()}
        contextActions={<button type="button">Collection options</button>}
      />,
    );

    expect(screen.getByRole("button", { name: "Collection options" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add command" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Quick Add" })).not.toBeInTheDocument();
  });

  it("makes create and manage collection actions explicit", async () => {
    const user = userEvent.setup();
    const onCreateCollection = vi.fn();
    const onManageCollections = vi.fn();

    render(
      <Sidebar
        stats={{ total: 3, favorites: 1, recent: 2, scripts: 0 }}
        tags={[]}
        collections={[]}
        scope={{ type: "all" }}
        view="library"
        onScopeChange={vi.fn()}
        onOpenSettings={vi.fn()}
        onCreateCollection={onCreateCollection}
        onManageCollections={onManageCollections}
        onDismiss={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Create collection" }));
    await user.click(screen.getByRole("button", { name: "Manage collections" }));

    expect(onCreateCollection).toHaveBeenCalledOnce();
    expect(onManageCollections).toHaveBeenCalledOnce();
    expect(
      screen.getByText("Create a collection to keep related commands together."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Arch Rescue/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Import" })).not.toBeInTheDocument();
  });
});
