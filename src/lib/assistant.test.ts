import { describe, expect, it } from "vitest";

import { MAX_TURNS, lastQuestion, proposalToInput, toTurns } from "./assistant";
import type { AssistantMessage } from "./assistant";
import type { CommandProposal } from "./types";

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

function message(overrides: Partial<AssistantMessage> = {}): AssistantMessage {
  return {
    id: "user-1",
    role: "user",
    text: "how do I free disk space?",
    proposals: [],
    ...overrides,
  };
}

describe("assistant conversation state", () => {
  it("resends proposed commands so a follow-up can refer to them", () => {
    const turns = toTurns([
      message(),
      message({
        id: "assistant-1",
        role: "assistant",
        text: "Prune the stopped containers first.",
        proposals: [proposal()],
      }),
    ]);

    expect(turns).toEqual([
      { role: "user", text: "how do I free disk space?", commands: [] },
      {
        role: "assistant",
        text: "Prune the stopped containers first.",
        commands: ["docker container prune"],
      },
    ]);
  });

  it("leaves a failed exchange out of what gets resent", () => {
    const turns = toTurns([
      message({ id: "user-1", text: "answered" }),
      message({ id: "user-2", text: "never answered", failed: true }),
    ]);

    expect(turns).toHaveLength(1);
    expect(turns[0].text).toBe("answered");
  });

  it("sends only the newest turns", () => {
    const messages = Array.from({ length: MAX_TURNS + 5 }, (_, index) =>
      message({ id: `user-${index}`, text: `question ${index}` }),
    );

    const turns = toTurns(messages);

    expect(turns).toHaveLength(MAX_TURNS);
    expect(turns[turns.length - 1].text).toBe(`question ${messages.length - 1}`);
  });

  it("opens a proposal in the normal entry form, keeping the reviewed risk level", () => {
    const input = proposalToInput(proposal());

    expect(input.title).toBe("Remove stopped containers");
    expect(input.content).toBe("docker container prune");
    expect(input.description).toBe("Frees the disk space they were holding.");
    expect(input.kind).toBe("command");
    expect(input.shell).toBe("bash");
    expect(input.riskLevel).toBe("caution");
    // Nothing is chosen on the user's behalf beyond what they saw.
    expect(input.tags).toEqual([]);
    expect(input.collectionIds).toEqual([]);
    expect(input.favorite).toBe(false);
  });

  it("finds the question to retry along with the conversation before it", () => {
    const messages = [
      message({ id: "user-1", text: "first" }),
      message({ id: "assistant-1", role: "assistant", text: "an answer" }),
      message({ id: "user-2", text: "second", failed: true }),
    ];

    expect(lastQuestion(messages)).toEqual({
      message: "second",
      history: messages.slice(0, 2),
    });
    expect(lastQuestion([])).toBeNull();
  });
});
