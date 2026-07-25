import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { emptyCommandInput } from "../lib/types";
import type { Collection } from "../lib/types";
import { CommandForm } from "./CommandForm";

const collections: Collection[] = [
  {
    id: 7,
    name: "Docker Cleanup",
    description: "",
    commandCount: 3,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  },
];

function setup(overrides: Partial<Parameters<typeof CommandForm>[0]> = {}) {
  const onSubmit = vi.fn();
  const onClose = vi.fn();

  render(
    <CommandForm
      open
      mode="create"
      initial={emptyCommandInput()}
      collections={collections}
      tagSuggestions={["docker", "git"]}
      saving={false}
      error={null}
      onSubmit={onSubmit}
      onClose={onClose}
      {...overrides}
    />,
  );

  return { onSubmit, onClose };
}

describe("CommandForm", () => {
  it("refuses to save an empty command and says why", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();

    await user.click(screen.getByRole("button", { name: "Save command" }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Paste or type the command first")).toBeInTheDocument();
    expect(screen.getByLabelText("Command")).toHaveAttribute("aria-invalid", "true");
  });

  it("submits the basic fields without touching the advanced section", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();

    await user.type(screen.getByLabelText("Title"), "List open ports");
    await user.type(screen.getByLabelText("Command"), "lsof -i :3000");
    await user.type(screen.getByLabelText("Description"), "What is on that port");
    await user.click(screen.getByRole("button", { name: "Save command" }));

    expect(onSubmit).toHaveBeenCalledOnce();
    expect(onSubmit.mock.calls[0][0]).toMatchObject({
      title: "List open ports",
      content: "lsof -i :3000",
      description: "What is on that port",
      kind: "command",
      riskLevel: null,
    });
  });

  it("collects tags typed with Enter", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();

    await user.type(screen.getByLabelText("Command"), "docker ps");
    await user.type(screen.getByLabelText("Tags"), "Docker{Enter}inspect{Enter}");
    await user.click(screen.getByRole("button", { name: "Save command" }));

    expect(onSubmit.mock.calls[0][0].tags).toEqual(["docker", "inspect"]);
  });

  it("keeps the extra details hidden until asked for", async () => {
    const user = userEvent.setup();
    setup();

    expect(screen.queryByLabelText("Shell")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Add extra details" }));

    expect(screen.getByLabelText("Shell")).toBeInTheDocument();
    expect(screen.getByLabelText("Risk")).toBeInTheDocument();
    expect(screen.getByLabelText("Docker Cleanup")).toBeInTheDocument();
  });

  it("sends the advanced values it was given", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();

    await user.type(screen.getByLabelText("Command"), "rm -rf ./dist");
    await user.click(screen.getByRole("button", { name: "Add extra details" }));
    await user.selectOptions(screen.getByLabelText("Risk"), "destructive");
    await user.type(screen.getByLabelText("Shell"), "fish");
    await user.click(screen.getByLabelText("Docker Cleanup"));
    await user.click(screen.getByRole("button", { name: "Save command" }));

    expect(onSubmit.mock.calls[0][0]).toMatchObject({
      riskLevel: "destructive",
      shell: "fish",
      collectionIds: [7],
    });
  });

  it("shows a save failure coming back from the backend", () => {
    setup({ error: "A collection named that already exists" });
    expect(screen.getByRole("alert")).toHaveTextContent("A collection named that already exists");
  });
});
