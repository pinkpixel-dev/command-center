import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { toDrafts } from "../../lib/import";
import type { CandidateDraft, ImportCandidate } from "../../lib/import";
import { ImportCandidateCard } from "./ImportCandidateCard";

function makeDraft(overrides: Partial<ImportCandidate> = {}): CandidateDraft {
  const candidate: ImportCandidate = {
    id: "candidate-0",
    title: "Remove stopped containers",
    content: "docker container prune",
    description: "Frees the disk space dead containers hold",
    kind: "command",
    language: null,
    shell: "bash",
    tags: ["docker"],
    riskLevel: "caution",
    riskReasons: ["Removes Docker resources"],
    variables: [],
    headingPath: ["Docker", "Cleanup"],
    sourceLine: 14,
    looksLikeOutput: false,
    outputReason: null,
    droppedOutputLines: 0,
    duplicate: null,
    repeatedInDocument: false,
    selected: true,
    ...overrides,
  };
  return toDrafts([candidate])[0];
}

function setup(draft: CandidateDraft, extra: Partial<Parameters<typeof ImportCandidateCard>[0]> = {}) {
  const handlers = {
    onChange: vi.fn(),
    onContentCommitted: vi.fn(),
    onSplit: vi.fn(),
    onMergeUp: vi.fn(),
    onRemove: vi.fn(),
  };

  render(
    <ImportCandidateCard
      draft={draft}
      index={0}
      canMergeUp={false}
      tagSuggestions={["docker", "git"]}
      {...handlers}
      {...extra}
    />,
  );

  return handlers;
}

describe("ImportCandidateCard", () => {
  it("shows what was parsed and where it came from", () => {
    setup(makeDraft());

    expect(screen.getByDisplayValue("Remove stopped containers")).toBeInTheDocument();
    expect(screen.getByDisplayValue("docker container prune")).toBeInTheDocument();
    expect(screen.getByText("Docker › Cleanup")).toBeInTheDocument();
    expect(screen.getByText("line 14")).toBeInTheDocument();
    expect(screen.getByText("Caution")).toBeInTheDocument();
  });

  it("reports selection changes", async () => {
    const user = userEvent.setup();
    const { onChange } = setup(makeDraft());

    await user.click(screen.getByRole("checkbox", { name: /Import Remove stopped containers/ }));
    expect(onChange).toHaveBeenCalledWith({ selected: false });
  });

  it("re-analyses when the command text is committed", async () => {
    const user = userEvent.setup();
    const { onContentCommitted } = setup(makeDraft());

    const field = screen.getByLabelText("Command for entry 1");
    await user.click(field);
    await user.tab();

    expect(onContentCommitted).toHaveBeenCalled();
  });

  it("explains an output block instead of hiding it", () => {
    setup(
      makeDraft({
        looksLikeOutput: true,
        outputReason: "Starts with a familiar output header",
        selected: false,
      }),
    );

    expect(screen.getByRole("note")).toHaveTextContent("Looks like terminal output");
    expect(screen.getByRole("checkbox", { name: /Import/ })).not.toBeChecked();
  });

  it("offers the four duplicate choices, defaulting to skip", async () => {
    const user = userEvent.setup();
    const { onChange } = setup(makeDraft({ duplicate: { id: 4, title: "Prune containers" } }));

    expect(screen.getByText(/Already saved as "Prune containers"/)).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Skip" })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: "Merge" }));
    expect(onChange).toHaveBeenCalledWith({ duplicateAction: "merge" });
  });

  it("only offers Split when there is more than one line", () => {
    setup(makeDraft());
    expect(screen.queryByRole("button", { name: /Split/ })).not.toBeInTheDocument();

    setup(makeDraft({ id: "multi", content: "npm test\nnpm run build" }));
    expect(screen.getAllByRole("button", { name: /Split/ }).length).toBeGreaterThan(0);
  });

  it("only offers Merge up when something sits above it", () => {
    setup(makeDraft(), { canMergeUp: true });
    expect(screen.getByRole("button", { name: /Merge up/ })).toBeInTheDocument();
  });

  it("mentions output lines that were stripped from a session", () => {
    setup(makeDraft({ droppedOutputLines: 3 }));
    expect(screen.getByText("3 output lines removed")).toBeInTheDocument();
  });

  it("lists detected placeholders", () => {
    setup(makeDraft({ content: "ssh {{user}}@{{host}}", variables: ["user", "host"] }));
    expect(screen.getByText("placeholders: {{user}} {{host}}")).toBeInTheDocument();
  });

  it("names the remove button after the entry", () => {
    setup(makeDraft());
    expect(
      screen.getByRole("button", { name: "Remove Remove stopped containers from the import" }),
    ).toBeInTheDocument();
  });
});
