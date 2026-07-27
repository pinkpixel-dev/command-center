import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { ExplanationView } from "../lib/types";
import { CommandExplanation } from "./CommandExplanation";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      getCommandExplanation: vi.fn(),
      explainCommand: vi.fn(),
    },
  };
});

const cached: ExplanationView = {
  commandId: 1,
  model: "gpt-5.6-luna",
  stale: false,
  generatedAt: "2026-07-26T10:00:00Z",
  explanation: {
    summary: "Deletes the build directory and everything inside it.",
    flags: [{ flag: "-r", meaning: "Recurse into directories" }],
    pipeline: [{ stage: "rm -r build", purpose: "Removes the directory tree" }],
    sideEffects: ["Files are removed from disk"],
    safety: {
      level: "caution",
      localReasons: ["Deletes files or directories"],
      aiReasons: ["No confirmation prompt"],
    },
    previewCommand: {
      command: "ls -la build",
      riskLevel: "safe",
      riskReasons: [],
    },
    assumptions: ["Run from the repository root"],
    caveats: ["Nothing here is recoverable from a recycle bin"],
  },
};

function setup(ready = true) {
  const onCopy = vi.fn();
  const view = render(<CommandExplanation commandId={1} ready={ready} onCopy={onCopy} />);
  return { onCopy, ...view };
}

describe("CommandExplanation", () => {
  beforeEach(() => {
    vi.mocked(api.getCommandExplanation).mockReset();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(null);
    vi.mocked(api.explainCommand).mockReset();
  });

  it("stays out of the dialog entirely while AI is unavailable", async () => {
    const { container } = setup(false);

    expect(container).toBeEmptyDOMElement();
    await waitFor(() => expect(api.getCommandExplanation).not.toHaveBeenCalled());
  });

  it("says what leaves the machine before anything is sent", async () => {
    setup();

    expect(
      await screen.findByRole("button", { name: /Explain this entry/ }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Ask AI what this entry does/)).toBeVisible();
    expect(api.explainCommand).not.toHaveBeenCalled();
  });

  it("loads a saved explanation without asking OpenAI again", async () => {
    vi.mocked(api.getCommandExplanation).mockResolvedValue(cached);
    setup();

    expect(
      await screen.findByText("Deletes the build directory and everything inside it."),
    ).toBeVisible();
    expect(api.explainCommand).not.toHaveBeenCalled();
    expect(screen.getByText(/AI-generated results may be inaccurate/)).toBeVisible();
    expect(screen.getByText(/Written by gpt-5.6-luna/)).toBeVisible();
  });

  it("fills the Quick and Detailed views from one result", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(cached);
    setup();

    await screen.findByText(cached.explanation.summary);
    expect(screen.queryByText("Recurse into directories")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Detailed" }));

    expect(screen.getByText("Recurse into directories")).toBeVisible();
    expect(screen.getByText("Removes the directory tree")).toBeVisible();
    expect(screen.getByText("Files are removed from disk")).toBeVisible();
    expect(screen.getByText("Run from the repository root")).toBeVisible();
    expect(screen.getByText(/recycle bin/)).toBeVisible();
    // Both views came from the one cached read.
    expect(api.getCommandExplanation).toHaveBeenCalledOnce();
    expect(api.explainCommand).not.toHaveBeenCalled();
  });

  it("keeps local reasons and model reasons apart in the safety review", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(cached);
    setup();

    await screen.findByText(cached.explanation.summary);
    await user.click(screen.getByRole("button", { name: "Detailed" }));

    const local = screen.getByText(/Deletes files or directories/).closest("li");
    const fromModel = screen.getByText(/No confirmation prompt/).closest("li");

    expect(local).toHaveTextContent("Command Center Deletes files or directories");
    expect(fromModel).toHaveTextContent("OpenAI No confirmation prompt");
  });

  it("offers a proposed preview to copy and never runs or saves it", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(cached);
    const { onCopy } = setup();

    await screen.findByText(cached.explanation.summary);
    await user.click(screen.getByRole("button", { name: "Detailed" }));
    await user.click(screen.getByRole("button", { name: /Copy preview/ }));

    expect(onCopy).toHaveBeenCalledWith("ls -la build");
    expect(screen.queryByRole("button", { name: /Run/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Save/ })).not.toBeInTheDocument();
  });

  it("marks an explanation stale once the entry changes and offers a refresh", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue({ ...cached, stale: true });
    vi.mocked(api.explainCommand).mockResolvedValue(cached);
    setup();

    expect(await screen.findByText(/This entry changed after the explanation/)).toBeVisible();
    expect(screen.getByText(/refresh the explanation to see the latest version/)).toBeVisible();

    await user.click(screen.getByRole("button", { name: /Refresh/ }));

    expect(api.explainCommand).toHaveBeenCalledWith(1);
    await waitFor(() =>
      expect(screen.queryByText(/This entry changed after the explanation/)).not.toBeInTheDocument(),
    );
  });

  it("reports a failed request without losing what is already on screen", async () => {
    const user = userEvent.setup();
    vi.mocked(api.getCommandExplanation).mockResolvedValue(cached);
    vi.mocked(api.explainCommand).mockRejectedValue({
      kind: "ai_rate_limit",
      message: "OpenAI is rate limiting this key. Try again shortly.",
    });
    setup();

    await screen.findByText(cached.explanation.summary);
    await user.click(screen.getByRole("button", { name: /Refresh/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("rate limiting");
    expect(screen.getByText(cached.explanation.summary)).toBeVisible();
  });

  it("generates on request and shows the result", async () => {
    const user = userEvent.setup();
    vi.mocked(api.explainCommand).mockResolvedValue(cached);
    setup();

    await user.click(await screen.findByRole("button", { name: /Explain this entry/ }));

    expect(api.explainCommand).toHaveBeenCalledWith(1);
    expect(await screen.findByText(cached.explanation.summary)).toBeVisible();
  });
});
