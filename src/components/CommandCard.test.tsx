import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { makeEntry } from "../test/factories";
import { CommandCard } from "./CommandCard";

function setup(props: Partial<Parameters<typeof CommandCard>[0]> = {}) {
  const handlers = {
    onOpenDetails: vi.fn(),
    onCopy: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onToggleFavorite: vi.fn(),
  };

  const view = render(
    <CommandCard
      entry={makeEntry()}
      viewMode="compact"
      {...handlers}
      {...props}
    />,
  );

  return { ...handlers, ...view };
}

describe("CommandCard", () => {
  it("shows the title and command without copy statistics", () => {
    setup();

    expect(screen.getByText("Update Arch packages")).toBeInTheDocument();
    expect(screen.getByText("sudo pacman -Syu")).toBeInTheDocument();
    expect(screen.queryByText(/Copied 18 times/)).not.toBeInTheDocument();
    expect(screen.getByText("#arch")).toBeInTheDocument();
  });

  it("uses a labelled icon for risk instead of visible badge text", () => {
    setup();

    const risk = screen.getByLabelText(/Caution: Runs with elevated privileges/);
    expect(risk).toHaveAttribute("title", expect.stringContaining("Caution"));
    expect(screen.queryByText("Caution")).not.toBeInTheDocument();
  });

  it("keeps safe entries free of a risk icon", () => {
    setup({ entry: makeEntry({ riskLevel: "safe", riskReasons: [] }) });
    expect(screen.queryByLabelText(/Risk: Safe/)).not.toBeInTheDocument();
  });

  it("opens the full-entry dialog from the title or code preview", async () => {
    const user = userEvent.setup();
    const { onOpenDetails } = setup();

    await user.click(screen.getByRole("button", { name: "Update Arch packages" }));
    await user.click(
      screen.getByRole("button", { name: "View full content for Update Arch packages" }),
    );

    expect(onOpenDetails).toHaveBeenCalledTimes(2);
  });

  it("copies the complete command from the icon action", async () => {
    const user = userEvent.setup();
    const { onCopy } = setup();

    await user.click(screen.getByRole("button", { name: "Copy Update Arch packages" }));
    expect(onCopy).toHaveBeenCalledWith("sudo pacman -Syu");
  });

  it("exposes favorite as a pressed toggle", async () => {
    const user = userEvent.setup();
    const { onToggleFavorite } = setup();

    const star = screen.getByRole("button", { name: "Add to favorites" });
    expect(star).toHaveAttribute("aria-pressed", "false");

    await user.click(star);
    expect(onToggleFavorite).toHaveBeenCalledOnce();
  });

  it("uses accessible names and tooltips for icon-only actions", () => {
    setup();

    expect(screen.getByRole("button", { name: "Copy Update Arch packages" })).toHaveAttribute(
      "title",
      "Copy",
    );
    expect(screen.getByRole("button", { name: "Edit Update Arch packages" })).toHaveAttribute(
      "title",
      "Edit",
    );
    expect(screen.getByRole("button", { name: "Delete Update Arch packages" })).toHaveAttribute(
      "title",
      "Delete",
    );
  });

  it("marks the selected presentation mode for fixed responsive styling", () => {
    const { container, rerender } = setup();
    expect(container.querySelector("article")).toHaveClass("card--compact");

    rerender(
      <CommandCard
        entry={makeEntry()}
        viewMode="cards"
        onOpenDetails={vi.fn()}
        onCopy={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleFavorite={vi.fn()}
      />,
    );

    expect(container.querySelector("article")).toHaveClass("card--cards");
  });
});
