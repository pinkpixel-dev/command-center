import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { Tag } from "../lib/types";
import { TagsBrowser } from "./TagsBrowser";

const tags: Tag[] = [
  { id: 1, name: "git", commandCount: 12 },
  { id: 2, name: "docker", commandCount: 4 },
];

describe("TagsBrowser", () => {
  it("presents every tag and selects one by name", async () => {
    const user = userEvent.setup();
    const onSelectTag = vi.fn();

    render(<TagsBrowser tags={tags} activeTagName="docker" onSelectTag={onSelectTag} />);

    expect(screen.getByRole("heading", { name: "Tags" })).toBeVisible();
    expect(screen.getByText("2 tags")).toBeVisible();

    const active = screen.getByRole("button", { name: "docker 4 commands" });
    expect(active).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: "git 12 commands" }));
    expect(onSelectTag).toHaveBeenCalledWith("git");
  });

  it("has a useful empty state", () => {
    render(<TagsBrowser tags={[]} activeTagName={null} onSelectTag={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "No tags yet" })).toBeVisible();
    expect(screen.getByText(/as you add them to commands/i)).toBeVisible();
  });
});
