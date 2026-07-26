import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { makeEntry } from "../test/factories";
import { CommandCard } from "./CommandCard";

function setup(props: Partial<Parameters<typeof CommandCard>[0]> = {}) {
  const handlers = {
    onToggle: vi.fn(),
    onCopy: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onToggleFavorite: vi.fn(),
    onOpenSource: vi.fn(),
  };

  const view = render(
    <CommandCard
      entry={makeEntry()}
      viewMode="compact"
      expanded={false}
      {...handlers}
      {...props}
    />,
  );

  return { ...handlers, ...view };
}

describe("CommandCard", () => {
  it("shows the title, the command, and how often it was copied", () => {
    setup();

    expect(screen.getByText("Update Arch packages")).toBeInTheDocument();
    expect(screen.getByText("sudo pacman -Syu")).toBeInTheDocument();
    expect(screen.getByText(/Copied 18 times/)).toBeInTheDocument();
    expect(screen.getByText("#arch")).toBeInTheDocument();
  });

  it("labels risk with a word, not only a colour", () => {
    setup();
    expect(screen.getByText("Caution")).toBeInTheDocument();
  });

  it("keeps safe entries free of a risk badge", () => {
    setup({ entry: makeEntry({ riskLevel: "safe", riskReasons: [] }) });
    expect(screen.queryByText("Safe")).not.toBeInTheDocument();
  });

  it("reports its expanded state to assistive tech and toggles on click", async () => {
    const user = userEvent.setup();
    const { onToggle } = setup();

    const toggle = screen.getByRole("button", { name: "Update Arch packages" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");

    await user.click(toggle);
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it("hides the details until it is expanded", () => {
    const { rerender } = setup();
    expect(screen.queryByText("Refresh every installed package")).not.toBeVisible();

    rerender(
      <CommandCard
        entry={makeEntry()}
        viewMode="compact"
        expanded
        onToggle={vi.fn()}
        onCopy={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleFavorite={vi.fn()}
        onOpenSource={vi.fn()}
      />,
    );
    expect(screen.getByText("Refresh every installed package")).toBeVisible();
    expect(screen.getByText("Runs with elevated privileges")).toBeVisible();
  });

  it("copies the command as written when it has no placeholders", async () => {
    const user = userEvent.setup();
    const { onCopy } = setup();

    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopy).toHaveBeenCalledWith("sudo pacman -Syu");
  });

  it("copies the filled-in version of a templated command", async () => {
    const user = userEvent.setup();
    const entry = makeEntry({
      id: 2,
      title: "SSH in",
      content: "ssh {{user}}@{{host}}",
      variables: ["user", "host"],
      riskLevel: "safe",
      riskReasons: [],
    });
    const { onCopy } = setup({ entry, expanded: true });

    await user.type(screen.getByLabelText("user"), "pinkpixel");
    await user.type(screen.getByLabelText("host"), "10.0.0.4");
    await user.click(screen.getByRole("button", { name: "Copy" }));

    expect(onCopy).toHaveBeenCalledWith("ssh pinkpixel@10.0.0.4");
  });

  it("exposes favorite as a pressed toggle", async () => {
    const user = userEvent.setup();
    const { onToggleFavorite } = setup();

    const star = screen.getByRole("button", { name: "Add to favorites" });
    expect(star).toHaveAttribute("aria-pressed", "false");

    await user.click(star);
    expect(onToggleFavorite).toHaveBeenCalledOnce();
  });

  it("names the delete action after the entry it removes", () => {
    setup();
    expect(
      screen.getByRole("button", { name: "Delete Update Arch packages" }),
    ).toBeInTheDocument();
  });

  it("marks the selected presentation mode for responsive styling", () => {
    const { container, rerender } = setup();
    expect(container.querySelector("article")).toHaveClass("card--compact");

    rerender(
      <CommandCard
        entry={makeEntry()}
        viewMode="cards"
        expanded={false}
        onToggle={vi.fn()}
        onCopy={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleFavorite={vi.fn()}
        onOpenSource={vi.fn()}
      />,
    );

    expect(container.querySelector("article")).toHaveClass("card--cards");
  });
});
