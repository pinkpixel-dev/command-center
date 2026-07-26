import { describe, expect, it } from "vitest";

import {
  buildImportItems,
  countDrafts,
  mergeCandidates,
  splitCandidate,
  summarize,
  toDrafts,
  withAnalysis,
} from "./import";
import type { CandidateDraft, ImportCandidate, SnippetAnalysis } from "./import";

function candidate(overrides: Partial<ImportCandidate> = {}): ImportCandidate {
  return {
    id: "candidate-0",
    title: "Show containers",
    content: "docker ps",
    description: "Lists running containers",
    kind: "command",
    language: null,
    shell: "bash",
    tags: ["docker"],
    riskLevel: "safe",
    riskReasons: [],
    variables: [],
    headingPath: ["Docker"],
    sourceLine: 12,
    looksLikeOutput: false,
    outputReason: null,
    droppedOutputLines: 0,
    duplicate: null,
    repeatedInDocument: false,
    selected: true,
    ...overrides,
  };
}

function draft(overrides: Partial<CandidateDraft> = {}): CandidateDraft {
  return { ...toDrafts([candidate()])[0], ...overrides };
}

describe("toDrafts", () => {
  it("defaults duplicates to skipping and everything else to creating", () => {
    const drafts = toDrafts([
      candidate({ id: "a" }),
      candidate({ id: "b", duplicate: { id: 7, title: "Already saved" } }),
    ]);

    expect(drafts[0].duplicateAction).toBe("create");
    expect(drafts[1].duplicateAction).toBe("skip");
  });

  it("applies the default collection to every draft", () => {
    const drafts = toDrafts([candidate({ id: "a" }), candidate({ id: "b" })], [3]);
    expect(drafts.every((entry) => entry.collectionIds.includes(3))).toBe(true);
  });
});

describe("splitCandidate", () => {
  it("makes one draft per line", () => {
    const pieces = splitCandidate(
      draft({ content: "npm test\nnpm run build\nnpm publish", kind: "sequence" }),
    );

    expect(pieces).toHaveLength(3);
    expect(pieces.map((piece) => piece.content)).toEqual([
      "npm test",
      "npm run build",
      "npm publish",
    ]);
    expect(pieces.every((piece) => piece.kind === "command")).toBe(true);
    expect(new Set(pieces.map((piece) => piece.id)).size).toBe(3);
  });

  it("keeps the description on the first piece only", () => {
    const pieces = splitCandidate(draft({ content: "a\nb", description: "Shared context" }));
    expect(pieces[0].description).toBe("Shared context");
    expect(pieces[1].description).toBe("");
  });

  it("leaves a single-line candidate alone", () => {
    const single = draft({ content: "docker ps" });
    expect(splitCandidate(single)).toEqual([single]);
  });

  it("ignores blank lines", () => {
    const pieces = splitCandidate(draft({ content: "npm test\n\n\nnpm run build\n" }));
    expect(pieces).toHaveLength(2);
  });
});

describe("mergeCandidates", () => {
  it("stacks the contents and unions the tags", () => {
    const merged = mergeCandidates(
      draft({ content: "npm test", tags: ["npm"], collectionIds: [1] }),
      draft({ id: "second", content: "npm run build", tags: ["release", "npm"], collectionIds: [2] }),
    );

    expect(merged.content).toBe("npm test\nnpm run build");
    expect(merged.tags).toEqual(["npm", "release"]);
    expect(merged.collectionIds).toEqual([1, 2]);
    expect(merged.kind).toBe("sequence");
  });

  it("keeps the first description when there is one", () => {
    const merged = mergeCandidates(
      draft({ description: "First" }),
      draft({ id: "second", description: "Second" }),
    );
    expect(merged.description).toBe("First");
  });

  it("borrows the second description when the first is empty", () => {
    const merged = mergeCandidates(
      draft({ description: "" }),
      draft({ id: "second", description: "Second" }),
    );
    expect(merged.description).toBe("Second");
  });
});

describe("withAnalysis", () => {
  const analysis: SnippetAnalysis = {
    kind: "sequence",
    riskLevel: "destructive",
    riskReasons: ["Recursive force delete"],
    variables: ["path"],
    looksLikeOutput: false,
    outputReason: null,
    duplicate: { id: 4, title: "Saved already" },
    suggestedTitle: "rm -rf {{path}}",
  };

  it("takes the recalculated facts", () => {
    const updated = withAnalysis(draft(), analysis);
    expect(updated.riskLevel).toBe("destructive");
    expect(updated.kind).toBe("sequence");
    expect(updated.variables).toEqual(["path"]);
    expect(updated.duplicate?.title).toBe("Saved already");
    expect(updated.duplicateAction).toBe("skip");
  });

  it("keeps a title the user typed but fills an empty one", () => {
    expect(withAnalysis(draft({ title: "My title" }), analysis).title).toBe("My title");
    expect(withAnalysis(draft({ title: "  " }), analysis).title).toBe("rm -rf {{path}}");
  });
});

describe("buildImportItems", () => {
  it("only includes ticked drafts", () => {
    const items = buildImportItems([
      draft({ id: "a", selected: true, title: "Keep" }),
      draft({ id: "b", selected: false, title: "Drop" }),
    ]);

    expect(items).toHaveLength(1);
    expect(items[0].input.title).toBe("Keep");
  });

  it("carries the duplicate choice and the entry it points at", () => {
    const items = buildImportItems([
      draft({
        duplicate: { id: 9, title: "Existing" },
        duplicateAction: "merge",
      }),
    ]);

    expect(items[0].duplicateAction).toBe("merge");
    expect(items[0].existingId).toBe(9);
  });

  it("never sends a duplicate action for something that is not a duplicate", () => {
    const items = buildImportItems([draft({ duplicate: null, duplicateAction: "replace" })]);
    expect(items[0].duplicateAction).toBe("create");
    expect(items[0].existingId).toBeNull();
  });

  it("passes tags, kind, and collections through", () => {
    const items = buildImportItems([
      draft({ tags: ["docker", "cleanup"], kind: "script", collectionIds: [2, 5] }),
    ]);

    expect(items[0].input.tags).toEqual(["docker", "cleanup"]);
    expect(items[0].input.kind).toBe("script");
    expect(items[0].input.collectionIds).toEqual([2, 5]);
  });
});

describe("countDrafts", () => {
  it("counts what will actually happen", () => {
    const counts = countDrafts([
      draft({ id: "a", selected: true }),
      draft({ id: "b", selected: true, duplicate: { id: 1, title: "x" }, duplicateAction: "skip" }),
      draft({ id: "c", selected: false, looksLikeOutput: true }),
    ]);

    expect(counts.total).toBe(3);
    expect(counts.selected).toBe(2);
    expect(counts.duplicates).toBe(1);
    expect(counts.output).toBe(1);
    expect(counts.willSkip).toBe(1);
    expect(counts.willCreate).toBe(1);
  });
});

describe("summarize", () => {
  it("reads like a sentence", () => {
    expect(
      summarize({ created: 4, replaced: 1, merged: 0, skipped: 2, failures: [] }),
    ).toBe("4 added, 1 replaced, 2 skipped");
  });

  it("says so when nothing happened", () => {
    expect(summarize({ created: 0, replaced: 0, merged: 0, skipped: 0, failures: [] })).toBe(
      "Nothing to import",
    );
  });

  it("mentions failures", () => {
    expect(
      summarize({
        created: 1,
        replaced: 0,
        merged: 0,
        skipped: 0,
        failures: [{ title: "Broken", message: "empty" }],
      }),
    ).toBe("1 added, 1 failed");
  });
});
