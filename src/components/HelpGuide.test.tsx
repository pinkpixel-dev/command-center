import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { HelpGuide } from "./HelpGuide";

describe("HelpGuide", () => {
  it("explains the local library, completed AI tools, and privacy boundaries", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();

    render(<HelpGuide open onClose={onClose} />);

    expect(screen.getByRole("dialog", { name: "Help" })).toBeInTheDocument();
    expect(screen.getByText("ssh {{user}}@{{host}}")).toBeVisible();
    expect(screen.getByText(/never saved to the library/i)).toBeVisible();
    expect(screen.getByText(/system clipboard/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "AI is optional" })).toBeVisible();
    expect(screen.getByText(/stored by your operating system/i)).toBeVisible();
    expect(screen.getByText(/not stored by OpenAI through the API/i)).toBeVisible();
    expect(screen.getByText(/no detector can promise/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Import with review" })).toBeVisible();
    expect(screen.getByText(/Nothing is saved until you import it/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Explain a saved entry" })).toBeVisible();
    expect(screen.getByText(/marks its explanation stale/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Ask the assistant" })).toBeVisible();
    expect(screen.getByText(/never runs or saves a command on its own/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Analyze terminal errors" })).toBeVisible();
    expect(screen.getByText(/what evidence may be missing/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Convert between shells" })).toBeVisible();
    expect(screen.getByText(/never guaranteed equivalents/i)).toBeVisible();

    await user.click(screen.getByRole("button", { name: /^Close$/ }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
