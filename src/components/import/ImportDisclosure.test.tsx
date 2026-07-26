import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { AiImportPlan } from "../../lib/ai-import";
import { ImportDisclosure } from "./ImportDisclosure";

function plan(overrides: Partial<AiImportPlan> = {}): AiImportPlan {
  return {
    documentBytes: 18_500,
    sentBytes: 18_432,
    lineCount: 210,
    model: "gpt-5.6-luna",
    findings: [],
    ...overrides,
  };
}

describe("import disclosure", () => {
  it("says what will be sent before anything is sent", () => {
    render(
      <ImportDisclosure
        plan={plan()}
        sourceName="docker.md"
        busy={false}
        onSend={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByText(/sends 18 KB of text to OpenAI/i)).toBeInTheDocument();
    expect(screen.getByText(/gpt-5.6-luna/)).toBeInTheDocument();
    expect(screen.getByText(/docker\.md/)).toBeInTheDocument();
    expect(screen.getByText(/No likely secrets were found/i)).toBeInTheDocument();
  });

  it("reports redactions by location and never shows a value", async () => {
    const user = userEvent.setup();
    render(
      <ImportDisclosure
        plan={plan({
          findings: [
            { kind: "openai_api_key", placeholder: "{{OPENAI_API_KEY}}", line: 12 },
            { kind: "password", placeholder: "{{PASSWORD}}", line: 40 },
            { kind: "password", placeholder: "{{PASSWORD}}", line: 41 },
          ],
        })}
        sourceName={null}
        busy={false}
        onSend={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByText("3 likely secrets were replaced with placeholders.")).toBeInTheDocument();

    const toggle = screen.getByRole("button", { name: "Review what was replaced" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await user.click(toggle);

    expect(screen.getByText("OpenAI API key")).toBeInTheDocument();
    expect(screen.getByText("line 12")).toBeInTheDocument();
    expect(screen.getByText("lines 40, 41")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Hide what was replaced" }),
    ).toHaveAttribute("aria-expanded", "true");
  });

  it("sending is an explicit action, and backing out sends nothing", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    const onCancel = vi.fn();

    render(
      <ImportDisclosure
        plan={plan()}
        sourceName="notes.md"
        busy={false}
        onSend={onSend}
        onCancel={onCancel}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSend).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Send to OpenAI" }));
    expect(onSend).toHaveBeenCalledOnce();
  });

  it("shows the request in flight without offering a second send", () => {
    render(
      <ImportDisclosure
        plan={plan()}
        sourceName="notes.md"
        busy
        onSend={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: /Send to OpenAI/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
  });
});
