import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../../lib/ipc";
import type { OutboundPlan } from "../../lib/outbound";
import type { ErrorAnalysis } from "../../lib/types";
import { AssistantDock } from "./AssistantDock";

vi.mock("../../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      askAssistant: vi.fn(),
      cancelAssistantRequest: vi.fn(),
      prepareErrorAnalysis: vi.fn(),
      analyzeTerminalError: vi.fn(),
    },
  };
});

const PASTE = "$ npm run build\nError: connect ECONNREFUSED 127.0.0.1:5432\n  at TCPConnectWrap";

const plan: OutboundPlan = {
  documentBytes: 120,
  sentBytes: 104,
  lineCount: 3,
  model: "gpt-5.6-luna",
  findings: [{ kind: "password", placeholder: "{{PASSWORD}}", line: 2 }],
};

const cleanPlan: OutboundPlan = { ...plan, sentBytes: 120, findings: [] };

const analysis: ErrorAnalysis = {
  summary: "Postgres refused the connection on port 5432.",
  cause: "Nothing is listening on that port, so the database is probably not running.",
  confidence: "medium",
  relevantLines: [
    { line: 2, quote: "Error: connect ECONNREFUSED 127.0.0.1:5432", why: "The refused connection" },
  ],
  checks: [{ check: "Check whether Postgres is running", why: "Rules out a stopped service" }],
  proposals: [
    {
      command: "systemctl status postgresql",
      title: "Check the Postgres service",
      why: "Shows whether it is running at all.",
      kind: "command",
      shell: "bash",
      riskLevel: "safe",
      localReasons: [],
      aiReasons: [],
    },
  ],
  uncertainty: "The lines above the paste would show what tried to connect.",
};

function setup() {
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
      initialMode="paste"
      ready
      model="gpt-5.6-luna"
      {...handlers}
    />,
  );
  return { ...handlers, ...view };
}

async function paste(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByLabelText("Terminal output"));
  await user.paste(PASTE);
  await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));
}

describe("terminal error analysis", () => {
  beforeEach(() => {
    vi.mocked(api.prepareErrorAnalysis).mockReset().mockResolvedValue(plan);
    vi.mocked(api.analyzeTerminalError).mockReset().mockResolvedValue(analysis);
    vi.mocked(api.cancelAssistantRequest).mockReset().mockResolvedValue(true);
  });

  it("sends nothing until the disclosure has been shown and accepted", async () => {
    const user = userEvent.setup();
    setup();

    await user.click(screen.getByLabelText("Terminal output"));
    await user.paste(PASTE);
    expect(api.prepareErrorAnalysis).not.toHaveBeenCalled();
    expect(api.analyzeTerminalError).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));
    expect(api.prepareErrorAnalysis).toHaveBeenCalledWith(PASTE);
    expect(await screen.findByText("Ready to send")).toBeVisible();
    expect(api.analyzeTerminalError).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /Send to OpenAI/ }));
    expect(api.analyzeTerminalError).toHaveBeenCalledWith(expect.any(Number), PASTE);
  });

  /// The whole point of the disclosure: the size, the model, and every likely
  /// secret are on screen before the paste can go anywhere.
  it("says how much goes out, to which model, and what was replaced", async () => {
    const user = userEvent.setup();
    setup();
    await paste(user);

    expect(await screen.findByText(/104 bytes of text to OpenAI using gpt-5.6-luna/)).toBeVisible();
    expect(screen.getByText(/1 likely secret was replaced/)).toBeVisible();
    expect(screen.getByText("3 lines of output")).toBeVisible();

    await user.click(screen.getByRole("button", { name: /Review what was replaced/ }));
    expect(screen.getByText("Password")).toBeVisible();
    expect(screen.getByText("line 2")).toBeVisible();
    expect(screen.getByText("{{PASSWORD}}")).toBeVisible();
  });

  it("does not claim a secret was found when none was", async () => {
    const user = userEvent.setup();
    vi.mocked(api.prepareErrorAnalysis).mockResolvedValue(cleanPlan);
    setup();
    await paste(user);

    expect(await screen.findByText(/No likely secrets were found/)).toBeVisible();
    expect(screen.queryByRole("button", { name: /Review what was replaced/ })).toBeNull();
  });

  it("turns the analysis into a conversation with the cause, lines, and checks", async () => {
    const user = userEvent.setup();
    const { onErrorAnalyzed } = setup();

    await paste(user);
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    expect(onErrorAnalyzed).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "error", output: PASTE, analysis }),
    );
  });

  it("never offers to run a suggested command", async () => {
    const user = userEvent.setup();
    setup();

    await paste(user);
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    expect(screen.queryByRole("button", { name: /^Run/ })).toBeNull();
  });

  it("stops a request that is still on the wire", async () => {
    const user = userEvent.setup();
    let reject: (error: unknown) => void = () => undefined;
    vi.mocked(api.analyzeTerminalError).mockReturnValue(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    setup();

    await paste(user);
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    await user.click(await screen.findByRole("button", { name: "Stop" }));
    expect(api.cancelAssistantRequest).toHaveBeenCalledOnce();

    reject({ kind: "ai_cancelled", message: "Request cancelled." });
    expect(await screen.findByText(/Stopped. Nothing was analyzed./)).toBeVisible();
  });

  it("reports a failure without losing the disclosure", async () => {
    const user = userEvent.setup();
    vi.mocked(api.analyzeTerminalError).mockRejectedValue({
      kind: "ai_network",
      message: "No connection to OpenAI.",
    });
    setup();

    await paste(user);
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("No connection to OpenAI.");
    expect(screen.getByRole("button", { name: /Send to OpenAI/ })).toBeVisible();
  });

  it("stays out of the app entirely while AI is unavailable", () => {
    const { container } = render(
      <AssistantDock
        open
        context={null}
        initialMode="paste"
        ready={false}
        model="gpt-5.6-luna"
        onClose={vi.fn()}
        onCopy={vi.fn()}
        onReview={vi.fn()}
        onErrorAnalyzed={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
    expect(api.prepareErrorAnalysis).not.toHaveBeenCalled();
  });
});

describe("an analyzed error as a conversation", () => {
  beforeEach(() => {
    vi.mocked(api.askAssistant).mockReset();
    vi.mocked(api.cancelAssistantRequest).mockReset().mockResolvedValue(true);
  });

  function conversation() {
    return render(
      <AssistantDock
        open
        context={{
          kind: "error",
          sessionId: 1,
          output: PASTE,
          analysis,
          title: "Terminal error",
        }}
        initialMode="chat"
        ready
        model="gpt-5.6-luna"
        onClose={vi.fn()}
        onCopy={vi.fn()}
        onReview={vi.fn()}
        onErrorAnalyzed={vi.fn()}
      />,
    );
  }

  it("opens with the analysis already in the transcript", () => {
    conversation();

    expect(screen.getByText(/Postgres refused the connection/)).toBeVisible();
    expect(screen.getByText("Reading between the lines")).toBeVisible();
    expect(screen.getByText("Error: connect ECONNREFUSED 127.0.0.1:5432")).toBeVisible();
    expect(screen.getByText("Check whether Postgres is running")).toBeVisible();
    expect(screen.getByText(/would show what tried to connect/)).toBeVisible();
    expect(screen.getByText("systemctl status postgresql")).toBeVisible();
  });

  /// A follow-up has to carry the paste, or "which line said that?" has
  /// nothing to look at.
  it("resends the pasted output with a follow-up question", async () => {
    const user = userEvent.setup();
    vi.mocked(api.askAssistant).mockResolvedValue({ reply: "Line 2.", proposals: [] });
    conversation();

    await user.type(screen.getByLabelText("Message the assistant"), "which line says that?");
    await user.click(screen.getByRole("button", { name: "Send" }));

    expect(api.askAssistant).toHaveBeenCalledWith(
      expect.objectContaining({ commandId: null, errorOutput: PASTE }),
    );
  });

  it("says the paste travels with the message", () => {
    conversation();
    expect(screen.getByText(/Sent with the pasted output to gpt-5.6-luna/)).toBeVisible();
  });

  /// Switching away mid-thread would silently discard it, so the switch is
  /// gone once there is something to lose.
  it("hides the mode switch once a conversation exists", () => {
    conversation();
    expect(screen.queryByRole("group", { name: /What the assistant should do/ })).toBeNull();
  });
});
