import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import { makeEntry } from "../test/factories";
import { CommandDetailsDialog } from "./CommandDetailsDialog";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: { ...original.api, getCommandExplanation: vi.fn() },
  };
});

function setup(entry = makeEntry(), aiReady = false) {
  const handlers = {
    onClose: vi.fn(),
    onCopy: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onOpenSource: vi.fn(),
    onAskAssistant: vi.fn(),
  };
  const view = render(
    <CommandDetailsDialog entry={entry} open aiReady={aiReady} {...handlers} />,
  );
  return { ...handlers, ...view };
}

describe("CommandDetailsDialog", () => {
  it("shows complete details in a responsive full-entry dialog", () => {
    const { container } = setup();

    expect(screen.getByRole("dialog", { name: "Update Arch packages" })).toBeInTheDocument();
    expect(screen.getByText("Refresh every installed package")).toBeVisible();
    expect(screen.getByText("Runs with elevated privileges")).toBeVisible();
    expect(container.querySelector(".modal")).toHaveClass("modal--mobile-fullscreen");
  });

  it("keeps a long script complete inside the scrollable viewer", () => {
    const content = Array.from({ length: 80 }, (_, index) => `echo "line ${index + 1}"`).join("\n");
    const { container } = setup(
      makeEntry({
        title: "Long maintenance script",
        content,
        kind: "script",
        riskLevel: "safe",
        riskReasons: [],
      }),
    );

    expect(container.querySelector(".entry-dialog__code")?.textContent).toBe(content);
  });

  it("copies a filled-in template instead of the unresolved markers", async () => {
    const user = userEvent.setup();
    const { onCopy } = setup(
      makeEntry({
        id: 2,
        title: "SSH in",
        content: "ssh {{user}}@{{host}}",
        variables: ["user", "host"],
        riskLevel: "safe",
        riskReasons: [],
      }),
    );

    await user.type(screen.getByLabelText("user"), "pinkpixel");
    await user.type(screen.getByLabelText("host"), "10.0.0.4");
    await user.click(screen.getByRole("button", { name: "Copy" }));

    expect(onCopy).toHaveBeenCalledWith("ssh pinkpixel@10.0.0.4");
  });

  it("closes before handing off to edit or delete", async () => {
    const user = userEvent.setup();
    const { onClose, onEdit, onDelete } = setup();

    await user.click(screen.getByRole("button", { name: "Edit" }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onEdit).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(onClose).toHaveBeenCalledTimes(2);
    expect(onDelete).toHaveBeenCalledOnce();
  });

  it("keeps Explain out of the dialog until AI is on with a stored key", async () => {
    vi.mocked(api.getCommandExplanation).mockResolvedValue(null);
    const { rerender } = setup();

    expect(screen.queryByText("Explanation")).not.toBeInTheDocument();
    expect(api.getCommandExplanation).not.toHaveBeenCalled();

    rerender(
      <CommandDetailsDialog
        entry={makeEntry()}
        open
        aiReady
        onClose={vi.fn()}
        onCopy={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenSource={vi.fn()}
        onAskAssistant={vi.fn()}
      />,
    );

    expect(await screen.findByText("Explanation")).toBeVisible();
    expect(api.getCommandExplanation).toHaveBeenCalledWith(1);
  });

  it("hands the entry to the assistant and closes, once AI is available", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(null);

    const withoutAi = setup();
    expect(screen.queryByRole("button", { name: "Ask" })).not.toBeInTheDocument();
    withoutAi.unmount();

    const { onAskAssistant, onClose } = setup(makeEntry(), true);
    await user.click(screen.getByRole("button", { name: "Ask" }));

    expect(onClose).toHaveBeenCalledOnce();
    expect(onAskAssistant).toHaveBeenCalledOnce();
  });
});
