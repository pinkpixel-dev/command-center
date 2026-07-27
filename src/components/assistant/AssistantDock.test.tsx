import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../../lib/ipc";
import type { AssistantReply, CommandProposal } from "../../lib/types";
import { AssistantDock } from "./AssistantDock";

vi.mock("../../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...original,
    api: { ...original.api, askAssistant: vi.fn(), cancelAssistantRequest: vi.fn() },
  };
});

function proposal(overrides: Partial<CommandProposal> = {}): CommandProposal {
  return {
    command: "docker container prune",
    title: "Remove stopped containers",
    why: "Frees the disk space they were holding.",
    kind: "command",
    shell: "bash",
    riskLevel: "caution",
    localReasons: ["Removes Docker resources"],
    aiReasons: ["Cannot be undone"],
    ...overrides,
  };
}

const reply: AssistantReply = {
  reply: "Prune the stopped containers first.",
  proposals: [proposal()],
};

function setup(overrides: Partial<Parameters<typeof AssistantDock>[0]> = {}) {
  const handlers = {
    onClose: vi.fn(),
    onCopy: vi.fn(),
    onReview: vi.fn(),
    onErrorAnalyzed: vi.fn(),
  };
  const view = render(
    <AssistantDock
      open
      context={null}
      initialMode="chat"
      ready
      {...handlers}
      {...overrides}
    />,
  );
  return { ...handlers, ...view };
}

/** Request ids come from one window-wide counter, so the value is not fixed. */
function lastRequestId(): number {
  const calls = vi.mocked(api.askAssistant).mock.calls;
  return calls[calls.length - 1][0].requestId;
}

async function ask(user: ReturnType<typeof userEvent.setup>, text = "how do I free disk space?") {
  await user.type(screen.getByLabelText("Message the assistant"), text);
  await user.click(screen.getByRole("button", { name: "Send" }));
}

describe("AssistantDock", () => {
  beforeEach(() => {
    vi.mocked(api.askAssistant).mockReset();
    vi.mocked(api.cancelAssistantRequest).mockReset().mockResolvedValue(true);
  });

  it("stays out of the app entirely while AI is unavailable", () => {
    const { container } = setup({ ready: false });

    expect(container).toBeEmptyDOMElement();
    expect(api.askAssistant).not.toHaveBeenCalled();
  });

  it("carries the selected entry with the question", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    setup({ context: { kind: "entry", commandId: 7, title: "Clean up Docker" } });

    expect(screen.getByText("Clean up Docker")).toBeVisible();
    await ask(user, "is there a safer version?");

    expect(api.askAssistant).toHaveBeenCalledWith({
      requestId: expect.any(Number),
      commandId: 7,
      errorOutput: null,
      turns: [],
      message: "is there a safer version?",
    });
  });

  it("shows the answer, the proposal, and its risk without offering to run it", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    setup();

    await ask(user);

    expect(await screen.findByText("Prune the stopped containers first.")).toBeVisible();
    expect(screen.getByText("docker container prune")).toBeVisible();
    expect(screen.getByText("Caution")).toBeVisible();
    expect(screen.getByText(/Removes Docker resources/)).toBeVisible();
    expect(screen.getByText(/Cannot be undone/)).toBeVisible();
    expect(screen.queryByRole("button", { name: /Run/ })).not.toBeInTheDocument();
  });

  it("copies a proposal and hands Review and save to the entry form", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    const { onCopy, onReview } = setup();

    await ask(user);
    await screen.findByText("Prune the stopped containers first.");

    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopy).toHaveBeenCalledWith("docker container prune");

    await user.click(screen.getByRole("button", { name: "Review and save" }));
    expect(onReview).toHaveBeenCalledWith(proposal());
  });

  it("sends the conversation so far with a follow-up", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    setup();

    await ask(user);
    await screen.findByText("Prune the stopped containers first.");
    await ask(user, "what about images?");

    expect(api.askAssistant).toHaveBeenLastCalledWith({
      requestId: expect.any(Number),
      commandId: null,
      errorOutput: null,
      turns: [
        { role: "user", text: "how do I free disk space?", commands: [] },
        {
          role: "assistant",
          text: "Prune the stopped containers first.",
          commands: ["docker container prune"],
        },
      ],
      message: "what about images?",
    });
  });

  it("stops a request that is still running", async () => {
    const user = userEvent.setup();
    let reject: (error: unknown) => void = () => undefined;
    vi.mocked(api.askAssistant).mockReturnValue(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    setup();

    await ask(user);
    const stop = await screen.findByRole("button", { name: "Stop" });
    await user.click(stop);
    expect(api.cancelAssistantRequest).toHaveBeenCalledWith(lastRequestId());

    reject({ kind: "ai_cancelled", message: "Request cancelled." });

    expect(await screen.findByText(/Stopped/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Send" })).toBeInTheDocument();
  });

  it("reports a failure and offers to send the same question again", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant)
      .mockRejectedValueOnce({ kind: "ai_network", message: "No connection to OpenAI." })
      .mockResolvedValueOnce(reply);
    setup();

    await ask(user);
    expect(await screen.findByRole("alert")).toHaveTextContent("No connection to OpenAI.");

    await user.click(screen.getByRole("button", { name: /Send it again/ }));

    expect(await screen.findByText("Prune the stopped containers first.")).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(vi.mocked(api.askAssistant).mock.calls[1][0].turns).toEqual([]);
  });

  it("clears the conversation on request", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    setup();

    await ask(user);
    await screen.findByText("Prune the stopped containers first.");

    await user.click(screen.getByRole("button", { name: "Clear conversation" }));

    expect(screen.queryByText("Prune the stopped containers first.")).not.toBeInTheDocument();
    expect(screen.getByText("Ask for a command")).toBeVisible();
  });

  it("throws the conversation away when AI is turned off", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    const { rerender } = setup();

    await ask(user);
    await screen.findByText("Prune the stopped containers first.");

    rerender(
      <AssistantDock
        open
        context={null}
        initialMode="chat"
        ready={false}
        onClose={vi.fn()}
        onCopy={vi.fn()}
        onReview={vi.fn()}
        onErrorAnalyzed={vi.fn()}
      />,
    );
    rerender(
      <AssistantDock
        open
        context={null}
        initialMode="chat"
        ready
        onClose={vi.fn()}
        onCopy={vi.fn()}
        onReview={vi.fn()}
        onErrorAnalyzed={vi.fn()}
      />,
    );

    await waitFor(() =>
      expect(screen.queryByText("Prune the stopped containers first.")).not.toBeInTheDocument(),
    );
  });

  it("sends on Enter, keeps Shift+Enter for a new line, and closes on Escape", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue(reply);
    const { onClose } = setup();

    const composer = screen.getByLabelText("Message the assistant");
    expect(composer).toHaveFocus();

    await user.type(composer, "first line{Shift>}{Enter}{/Shift}second line");
    expect(api.askAssistant).not.toHaveBeenCalled();
    expect(composer).toHaveValue("first line\nsecond line");

    await user.type(composer, "{Enter}");
    expect(api.askAssistant).toHaveBeenCalledOnce();

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledOnce();
  });
});
