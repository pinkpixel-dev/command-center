import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { HelpGuide } from "./HelpGuide";

describe("HelpGuide", () => {
  it("explains the placeholder flow and temporary value boundary", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();

    render(<HelpGuide open onClose={onClose} />);

    expect(screen.getByRole("dialog", { name: "Help" })).toBeInTheDocument();
    expect(screen.getByText("ssh {{user}}@{{host}}")).toBeVisible();
    expect(screen.getByText(/never saved to the library/i)).toBeVisible();
    expect(screen.getByText(/system clipboard/i)).toBeVisible();

    await user.click(screen.getByRole("button", { name: /^Close$/ }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
