import { createRef } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

function collection(id: number) {
  return {
    id,
    name: `Collection ${id}`,
    description: "",
    commandCount: id,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  };
}

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

  it("can replace Add command with bulk selection controls", () => {
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
        contextActions={<button type="button">Finish selecting</button>}
        showAdd={false}
      />,
    );

    expect(screen.getByRole("button", { name: "Finish selecting" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Add command" })).not.toBeInTheDocument();
  });

  it("exposes collection management and global sidebar actions", async () => {
    const user = userEvent.setup();

    const onManageCollections = vi.fn();
    const onOpenPalette = vi.fn();
    const onOpenHelp = vi.fn();
    const onOpenShortcuts = vi.fn();
    const onOpenSettings = vi.fn();

    const { container } = render(
      <Sidebar
        stats={{ total: 3, favorites: 1, recent: 2, scripts: 0 }}
        tags={[]}
        collections={[]}
        scope={{ type: "all" }}
        view="library"
        importAvailable={false}
        assistantAvailable={false}
        assistantOpen={false}
        onScopeChange={vi.fn()}
        onOpenImport={vi.fn()}
        onOpenAssistant={vi.fn()}
        onOpenPalette={onOpenPalette}
        onOpenHelp={onOpenHelp}
        onOpenShortcuts={onOpenShortcuts}
        onOpenSettings={onOpenSettings}
        onManageCollections={onManageCollections}
        onDismiss={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Manage collections" }));
    await user.click(screen.getByRole("button", { name: "Command palette" }));
    await user.click(screen.getByRole("button", { name: "Help" }));
    await user.click(screen.getByRole("button", { name: "Keyboard shortcuts" }));
    await user.click(screen.getByRole("button", { name: "Settings" }));

    expect(onManageCollections).toHaveBeenCalledOnce();
    expect(onOpenPalette).toHaveBeenCalledOnce();
    expect(onOpenHelp).toHaveBeenCalledOnce();
    expect(onOpenShortcuts).toHaveBeenCalledOnce();
    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(
      screen.getByText("Create a collection to keep related commands together."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Arch Rescue/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Import" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Assistant" })).not.toBeInTheDocument();
    expect(container.querySelector(".sidebar__logo")).toHaveAttribute("src", "/logo.png");
    expect(screen.queryByText(/^v\d/)).not.toBeInTheDocument();
  });

  it("shows Import and the assistant only once AI is on with a stored key", async () => {
    const user = userEvent.setup();
    const onOpenImport = vi.fn();
    const onOpenAssistant = vi.fn();

    render(
      <Sidebar
        stats={{ total: 3, favorites: 1, recent: 2, scripts: 0 }}
        tags={[]}
        collections={[]}
        scope={{ type: "all" }}
        view="library"
        importAvailable
        assistantAvailable
        assistantOpen={false}
        onScopeChange={vi.fn()}
        onOpenImport={onOpenImport}
        onOpenAssistant={onOpenAssistant}
        onOpenPalette={vi.fn()}
        onOpenHelp={vi.fn()}
        onOpenShortcuts={vi.fn()}
        onOpenSettings={vi.fn()}
        onManageCollections={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Import" }));
    expect(onOpenImport).toHaveBeenCalledOnce();

    const assistant = screen.getByRole("button", { name: "Assistant" });
    expect(assistant).toHaveAttribute("aria-pressed", "false");
    await user.click(assistant);
    expect(onOpenAssistant).toHaveBeenCalledOnce();
  });

  it("limits organization links while keeping the active collection and tag visible", async () => {
    const user = userEvent.setup();
    const onViewAllCollections = vi.fn();
    const onViewAllTags = vi.fn();
    const collections = Array.from({ length: 8 }, (_, index) => collection(index + 1));
    const tags = Array.from({ length: 20 }, (_, index) => ({
      id: index + 1,
      name: `tag-${index + 1}`,
      commandCount: index + 1,
    }));

    const { rerender } = render(
      <Sidebar
        stats={{ total: 20, favorites: 1, recent: 2, scripts: 3 }}
        tags={tags}
        collections={collections}
        scope={{ type: "collection", id: 8 }}
        view="library"
        importAvailable={false}
        assistantAvailable={false}
        assistantOpen={false}
        onScopeChange={vi.fn()}
        onOpenImport={vi.fn()}
        onOpenAssistant={vi.fn()}
        onOpenPalette={vi.fn()}
        onOpenHelp={vi.fn()}
        onOpenShortcuts={vi.fn()}
        onOpenSettings={vi.fn()}
        onManageCollections={vi.fn()}
        onViewAllCollections={onViewAllCollections}
        onViewAllTags={onViewAllTags}
        onDismiss={vi.fn()}
      />,
    );

    expect(screen.getByText("Collection 8")).toBeVisible();
    expect(screen.queryByText("Collection 6")).not.toBeInTheDocument();
    expect(screen.getAllByText(/^Collection \d+$/)).toHaveLength(6);
    expect(screen.getByText("Collection 8").closest("button")).toHaveAttribute(
      "aria-current",
      "page",
    );

    rerender(
      <Sidebar
        stats={{ total: 20, favorites: 1, recent: 2, scripts: 3 }}
        tags={tags}
        collections={collections}
        scope={{ type: "tag", name: "tag-20" }}
        view="library"
        importAvailable={false}
        assistantAvailable={false}
        assistantOpen={false}
        onScopeChange={vi.fn()}
        onOpenImport={vi.fn()}
        onOpenAssistant={vi.fn()}
        onOpenPalette={vi.fn()}
        onOpenHelp={vi.fn()}
        onOpenShortcuts={vi.fn()}
        onOpenSettings={vi.fn()}
        onManageCollections={vi.fn()}
        onViewAllCollections={onViewAllCollections}
        onViewAllTags={onViewAllTags}
        onDismiss={vi.fn()}
      />,
    );

    expect(screen.getByText("tag-20")).toBeVisible();
    expect(screen.queryByText("tag-18")).not.toBeInTheDocument();
    expect(screen.getAllByText(/^tag-\d+$/)).toHaveLength(18);
    expect(screen.getByText("tag-20").closest("button")).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    await user.click(screen.getByRole("button", { name: "View all collections" }));
    await user.click(screen.getByRole("button", { name: "View all tags" }));
    expect(onViewAllCollections).toHaveBeenCalledOnce();
    expect(onViewAllTags).toHaveBeenCalledOnce();
  });
});
