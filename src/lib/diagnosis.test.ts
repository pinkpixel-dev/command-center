import { describe, expect, it } from "vitest";

import { analysisMessage, contextKey, toTurns } from "./assistant";
import type { AssistantContext } from "./assistant";
import { nextRequestId } from "./request-id";
import type { ErrorAnalysis } from "./types";

function analysis(overrides: Partial<ErrorAnalysis> = {}): ErrorAnalysis {
  return {
    summary: "Port 3000 is already in use.",
    cause: "Another process is bound to it.",
    confidence: "high",
    relevantLines: [{ line: 3, quote: "EADDRINUSE :::3000", why: "The bind failed" }],
    checks: [{ check: "Find what owns port 3000", why: "Tells you what to stop" }],
    proposals: [
      {
        command: "ss -ltnp",
        title: "List listening ports",
        why: "Shows what is bound.",
        kind: "command",
        shell: "bash",
        riskLevel: "safe",
        localReasons: [],
        aiReasons: [],
      },
    ],
    uncertainty: "",
    ...overrides,
  };
}

const errorContext: AssistantContext = {
  kind: "error",
  sessionId: 9,
  output: "Error: listen EADDRINUSE",
  analysis: analysis(),
  title: "Terminal error",
};

describe("contextKey", () => {
  it("tells one subject from another so a conversation never carries across", () => {
    expect(contextKey(null)).toBe("general");
    expect(contextKey({ kind: "entry", commandId: 4, title: "Deploy" })).toBe("entry-4");
    expect(contextKey({ kind: "entry", commandId: 5, title: "Deploy" })).toBe("entry-5");
    expect(contextKey(errorContext)).toBe("error-9");
  });

  /// Two pastes are two conversations even when the text happens to match.
  it("gives each analyzed paste its own key", () => {
    expect(contextKey({ ...errorContext, sessionId: 10 })).not.toBe(contextKey(errorContext));
  });
});

describe("analysisMessage", () => {
  it("opens the conversation with the analysis and its proposals", () => {
    const message = analysisMessage(analysis());

    expect(message.role).toBe("assistant");
    expect(message.analysis).toEqual(analysis());
    expect(message.proposals).toEqual(analysis().proposals);
  });

  /// The card is what the reader sees; the text is what a follow-up resends.
  /// They have to say the same thing, or the model answers about something
  /// the user cannot see.
  it("says in words what the card says in structure", () => {
    const message = analysisMessage(analysis());

    expect(message.text).toBe("Port 3000 is already in use. Another process is bound to it.");
  });

  it("survives an analysis with no cause written", () => {
    expect(analysisMessage(analysis({ cause: "" })).text).toBe("Port 3000 is already in use.");
  });

  /// The seeded message is real conversation history, so a follow-up carries
  /// it and the commands it proposed.
  it("becomes a turn that a follow-up resends", () => {
    expect(toTurns([analysisMessage(analysis())])).toEqual([
      {
        role: "assistant",
        text: "Port 3000 is already in use. Another process is bound to it.",
        commands: ["ss -ltnp"],
      },
    ]);
  });
});

describe("nextRequestId", () => {
  /// Rust tracks in-flight requests in one registry, and registering a
  /// duplicate id aborts whatever was already running under it. Every
  /// cancellable request in the window draws from this one counter.
  it("never issues the same id twice", () => {
    const ids = Array.from({ length: 50 }, () => nextRequestId());

    expect(new Set(ids).size).toBe(ids.length);
    expect([...ids].sort((a, b) => a - b)).toEqual(ids);
  });
});
