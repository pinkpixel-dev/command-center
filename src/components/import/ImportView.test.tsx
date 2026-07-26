import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AiImportPlan } from "../../lib/ai-import";
import type { ImportCandidate, ImportPreview } from "../../lib/import";
import { api } from "../../lib/ipc";
import { ImportView } from "./ImportView";

vi.mock("../../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      prepareAiImport: vi.fn(),
      runAiImport: vi.fn(),
      readImportDocument: vi.fn(),
      analyzeSnippet: vi.fn(),
      importCommands: vi.fn(),
    },
  };
});

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const plan: AiImportPlan = {
  documentBytes: 120,
  sentBytes: 108,
  lineCount: 6,
  model: "gpt-5.6-luna",
  findings: [{ kind: "password", placeholder: "{{PASSWORD}}", line: 4 }],
};

function candidate(overrides: Partial<ImportCandidate> = {}): ImportCandidate {
  return {
    id: "ai-candidate-0",
    title: "Prune containers",
    content: "docker container prune",
    description: "Frees disk space",
    kind: "command",
    language: null,
    shell: "bash",
    tags: ["docker"],
    riskLevel: "caution",
    riskReasons: ["Removes Docker resources"],
    aiRiskReasons: ["Deletes stopped containers"],
    aiNotes: "Assumes Docker is installed",
    variables: [],
    headingPath: [],
    sourceLine: 4,
    looksLikeOutput: false,
    outputReason: null,
    droppedOutputLines: 0,
    duplicate: null,
    repeatedInDocument: false,
    selected: true,
    ...overrides,
  };
}

const preview: ImportPreview = {
  sourceName: null,
  suggestedCollection: null,
  candidates: [candidate()],
  stats: { blocksFound: 1, commands: 1, outputBlocks: 0, duplicates: 0, repeated: 0 },
};

function renderView() {
  return render(
    <ImportView
      collections={[]}
      tagSuggestions={[]}
      onOpenLibrary={vi.fn()}
      onImported={vi.fn()}
    />,
  );
}

describe("AI-assisted import flow", () => {
  beforeEach(() => {
    vi.mocked(api.prepareAiImport).mockResolvedValue(plan);
    vi.mocked(api.runAiImport).mockResolvedValue(preview);
  });

  it("shows the disclosure before any request, then reviews what came back", async () => {
    const user = userEvent.setup();
    renderView();

    await user.type(screen.getByLabelText("Or paste content"), "docker container prune");
    await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));

    expect(await screen.findByText(/sends 108 bytes of text to OpenAI/i)).toBeInTheDocument();
    expect(api.runAiImport).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /Send to OpenAI/ }));

    expect(await screen.findByText(/1 of 1 selected/)).toBeInTheDocument();
    expect(api.runAiImport).toHaveBeenCalledWith("docker container prune", null);
    expect(screen.getByText(/These entries came from OpenAI and may be wrong/)).toBeInTheDocument();
  });

  it("labels model risk notes separately from the local ones", async () => {
    const user = userEvent.setup();
    renderView();

    await user.type(screen.getByLabelText("Or paste content"), "docker container prune");
    await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    expect(await screen.findByText(/From the model, and it may be wrong/)).toBeInTheDocument();
    expect(screen.getByText(/Risk notes: Deletes stopped containers/)).toBeInTheDocument();
    expect(screen.getByText(/Unsure about: Assumes Docker is installed/)).toBeInTheDocument();
  });

  it("backing out of the disclosure keeps the pasted document and sends nothing", async () => {
    const user = userEvent.setup();
    renderView();

    await user.type(screen.getByLabelText("Or paste content"), "docker container prune");
    await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));
    await user.click(await screen.findByRole("button", { name: "Back" }));

    expect(screen.getByLabelText("Or paste content")).toHaveValue("docker container prune");
    expect(api.runAiImport).not.toHaveBeenCalled();
  });

  it("reports a refused request without opening the review screen", async () => {
    const user = userEvent.setup();
    vi.mocked(api.runAiImport).mockRejectedValue({
      kind: "ai_refusal",
      message: "OpenAI declined the request.",
    });
    renderView();

    await user.type(screen.getByLabelText("Or paste content"), "docker container prune");
    await user.click(screen.getByRole("button", { name: /Check what would be sent/ }));
    await user.click(await screen.findByRole("button", { name: /Send to OpenAI/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("OpenAI declined the request.");
    expect(screen.queryByText(/of 1 selected/)).not.toBeInTheDocument();
  });
});
