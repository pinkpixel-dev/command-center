import { createRef } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

describe("library navigation", () => {
  it("keeps Add command as the only create action in the top bar", () => {
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
      />,
    );

    expect(screen.getByRole("button", { name: "Add command" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Quick Add" })).not.toBeInTheDocument();
  });

  it("keeps Settings visible while the dormant Import screen has no entry point", () => {
    render(
      <Sidebar
        stats={{ total: 3, favorites: 1, recent: 2, scripts: 0 }}
        tags={[]}
        collections={[]}
        scope={{ type: "all" }}
        view="library"
        onScopeChange={vi.fn()}
        onOpenSettings={vi.fn()}
        onManageCollections={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Import" })).not.toBeInTheDocument();
  });
});
