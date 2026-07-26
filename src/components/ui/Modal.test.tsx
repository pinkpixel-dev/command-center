import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Modal } from "./Modal";

describe("Modal", () => {
  it("renders nothing while closed", () => {
    render(
      <Modal open={false} title="Hidden" onClose={vi.fn()}>
        <p>Body</p>
      </Modal>,
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("is announced as a modal dialog with its title", () => {
    render(
      <Modal open title="Delete this command?" description="It cannot be undone" onClose={vi.fn()}>
        <p>Body</p>
      </Modal>,
    );

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAccessibleName("Delete this command?");
    expect(dialog).toHaveAccessibleDescription("It cannot be undone");
  });

  it("moves focus into the dialog when it opens", () => {
    render(
      <Modal open title="Focus me" onClose={vi.fn()}>
        <button type="button">Inside</button>
      </Modal>,
    );

    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Close dialog" }));
  });

  it("honors an explicitly requested initial focus target", () => {
    render(
      <Modal open title="Create collection" onClose={vi.fn()}>
        <input aria-label="Collection name" data-modal-autofocus />
      </Modal>,
    );

    expect(screen.getByLabelText("Collection name")).toHaveFocus();
  });

  it("closes on Escape and on the close button", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();

    render(
      <Modal open title="Closable" onClose={onClose}>
        <p>Body</p>
      </Modal>,
    );

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Close dialog" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("keeps Tab inside the dialog", async () => {
    const user = userEvent.setup();

    render(
      <Modal open title="Trapped" onClose={vi.fn()} footer={<button type="button">Last</button>}>
        <button type="button">First</button>
      </Modal>,
    );

    const close = screen.getByRole("button", { name: "Close dialog" });
    const first = screen.getByRole("button", { name: "First" });
    const last = screen.getByRole("button", { name: "Last" });

    expect(document.activeElement).toBe(close);
    await user.tab();
    expect(document.activeElement).toBe(first);
    await user.tab();
    expect(document.activeElement).toBe(last);
    // Wrapping around lands back on the first control instead of escaping.
    await user.tab();
    expect(document.activeElement).toBe(close);
  });
});
