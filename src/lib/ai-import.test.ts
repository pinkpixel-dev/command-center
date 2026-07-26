import { describe, expect, it } from "vitest";

import {
  describeOutbound,
  describeRedactions,
  formatBytes,
  groupRedactions,
  redactionLabel,
} from "./ai-import";
import type { AiImportPlan } from "./ai-import";

function plan(overrides: Partial<AiImportPlan> = {}): AiImportPlan {
  return {
    documentBytes: 18_432,
    sentBytes: 18_400,
    lineCount: 240,
    model: "gpt-5.6-luna",
    findings: [],
    ...overrides,
  };
}

describe("outbound request disclosure", () => {
  it("states the size that actually leaves the machine, not the file size", () => {
    const sentence = describeOutbound(plan({ documentBytes: 40_000, sentBytes: 18_400 }));

    expect(sentence).toContain("18 KB");
    expect(sentence).not.toContain("39 KB");
    expect(sentence).toContain("gpt-5.6-luna");
  });

  it("reads naturally at every size", () => {
    expect(formatBytes(0)).toBe("0 bytes");
    expect(formatBytes(900)).toBe("900 bytes");
    expect(formatBytes(2048)).toBe("2 KB");
    expect(formatBytes(1024 * 1024 * 3)).toBe("3.0 MB");
  });

  it("says nothing about redactions when nothing was found", () => {
    expect(describeRedactions(plan())).toBeNull();
  });

  it("counts redactions in singular and plural", () => {
    const single = plan({
      findings: [{ kind: "password", placeholder: "{{PASSWORD}}", line: 3 }],
    });
    const several = plan({
      findings: [
        { kind: "password", placeholder: "{{PASSWORD}}", line: 3 },
        { kind: "token", placeholder: "{{TOKEN}}", line: 9 },
      ],
    });

    expect(describeRedactions(single)).toBe("1 likely secret was replaced with a placeholder.");
    expect(describeRedactions(several)).toBe("2 likely secrets were replaced with placeholders.");
  });

  it("groups repeated findings by kind and keeps every line", () => {
    const grouped = groupRedactions(
      plan({
        findings: [
          { kind: "token", placeholder: "{{TOKEN}}", line: 4 },
          { kind: "password", placeholder: "{{PASSWORD}}", line: 8 },
          { kind: "token", placeholder: "{{TOKEN}}", line: 12 },
        ],
      }),
    );

    expect(grouped).toHaveLength(2);
    expect(grouped[0]).toMatchObject({ label: "Token or secret", lines: [4, 12] });
    expect(grouped[1]).toMatchObject({ label: "Password", lines: [8] });
  });

  it("labels an unknown detector kind without breaking the list", () => {
    expect(redactionLabel("private_key")).toBe("Private key");
    expect(redactionLabel("something_new")).toBe("Possible secret");
  });
});
