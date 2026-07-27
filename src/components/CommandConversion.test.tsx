import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Equivalence, ShellConversion, ShellOption } from "../lib/types";
import { CommandConversion } from "./CommandConversion";

vi.mock("../lib/ipc", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...original,
    api: {
      ...original.api,
      conversionShells: vi.fn(),
      convertCommandShell: vi.fn(),
      cancelAssistantRequest: vi.fn(),
    },
  };
});

const { api } = await import("../lib/ipc");

const SHELLS: ShellOption[] = [
  { id: "bash", label: "bash" },
  { id: "fish", label: "fish" },
  { id: "zsh", label: "zsh" },
  { id: "powershell", label: "PowerShell" },
];

function conversion(overrides: Partial<ShellConversion> = {}): ShellConversion {
  return {
    sourceShell: "bash",
    targetShell: "fish",
    original: "export API_HOST=example.com",
    converted: {
      command: "set -x API_HOST example.com",
      title: "Set the API host",
      why: "Sets the variable for this session.",
      kind: "command",
      shell: "fish",
      riskLevel: "safe",
      localReasons: [],
      aiReasons: [],
    },
    equivalence: "close",
    differences: ["fish scopes the variable to the session, not the process tree"],
    unsupported: [],
    notes: "",
    ...overrides,
  };
}

function setup(props: Partial<Parameters<typeof CommandConversion>[0]> = {}) {
  const handlers = { onCopy: vi.fn(), onReview: vi.fn() };
  const view = render(
    <CommandConversion commandId={4} entryShell="bash" ready {...handlers} {...props} />,
  );
  return { ...handlers, ...view };
}

async function convertTo(user: ReturnType<typeof userEvent.setup>, label: string) {
  await user.click(await screen.findByRole("button", { name: label }));
}

describe("CommandConversion", () => {
  beforeEach(() => {
    vi.mocked(api.conversionShells).mockReset().mockResolvedValue(SHELLS);
    vi.mocked(api.convertCommandShell).mockReset().mockResolvedValue(conversion());
    vi.mocked(api.cancelAssistantRequest).mockReset().mockResolvedValue(true);
  });

  it("stays out of the dialog entirely while AI is unavailable", () => {
    const { container } = setup({ ready: false });

    expect(container).toBeEmptyDOMElement();
    expect(api.conversionShells).not.toHaveBeenCalled();
  });

  /// The picker comes from the backend, so it cannot offer a shell the prompt
  /// and parser do not both know about.
  it("offers every supported shell except the one the entry already uses", async () => {
    setup();

    expect(await screen.findByRole("button", { name: "fish" })).toBeVisible();
    expect(screen.getByRole("button", { name: "zsh" })).toBeVisible();
    expect(screen.getByRole("button", { name: "PowerShell" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "bash" })).toBeNull();
  });

  it("offers all four when the entry does not name a shell", async () => {
    setup({ entryShell: null });

    for (const shell of ["bash", "fish", "zsh", "PowerShell"]) {
      expect(await screen.findByRole("button", { name: shell })).toBeVisible();
    }
  });

  it("shows both commands with what changed between them", async () => {
    const user = userEvent.setup();
    setup();

    await convertTo(user, "fish");

    expect(await screen.findByText("export API_HOST=example.com")).toBeVisible();
    expect(screen.getByText("set -x API_HOST example.com")).toBeVisible();
    expect(screen.getByText("Original")).toBeVisible();
    expect(screen.getByText("Converted")).toBeVisible();
    expect(screen.getByText(/fish scopes the variable/)).toBeVisible();
    expect(api.convertCommandShell).toHaveBeenCalledWith(expect.any(Number), 4, "fish");
  });

  /// The one thing this feature must never do is imply the rewrite is exact.
  it("never presents a conversion as a guaranteed equivalent", async () => {
    const user = userEvent.setup();
    const cases: [Equivalence, RegExp][] = [
      ["close", /Check the differences before you rely on it/],
      ["partial", /did not carry over/],
      ["uncertain", /may not be right. Test it before you use it/],
    ];

    for (const [equivalence, wording] of cases) {
      vi.mocked(api.convertCommandShell).mockResolvedValue(conversion({ equivalence }));
      const { unmount } = setup();

      await convertTo(user, "fish");
      expect(await screen.findByText(wording)).toBeVisible();
      for (const forbidden of [/identical/i, /guaranteed/i, /exactly equivalent/i]) {
        expect(screen.queryByText(forbidden)).toBeNull();
      }
      unmount();
    }
  });

  it("lists syntax that did not carry over", async () => {
    const user = userEvent.setup();
    vi.mocked(api.convertCommandShell).mockResolvedValue(
      conversion({
        equivalence: "partial",
        unsupported: ["fish has no equivalent of process substitution"],
      }),
    );
    setup();

    await convertTo(user, "fish");

    expect(await screen.findByText("What did not carry over")).toBeVisible();
    expect(screen.getByText(/no equivalent of process substitution/)).toBeVisible();
  });

  it("shows the local risk verdict on the converted command", async () => {
    const user = userEvent.setup();
    vi.mocked(api.convertCommandShell).mockResolvedValue(
      conversion({
        converted: {
          ...conversion().converted,
          command: "rm -rf ./dist",
          riskLevel: "destructive",
          localReasons: ["Recursively deletes files"],
          aiReasons: ["Cannot be undone"],
        },
      }),
    );
    setup();

    await convertTo(user, "fish");

    expect(await screen.findByText("Destructive")).toBeVisible();
    expect(screen.getByText(/Recursively deletes files/)).toBeVisible();
    expect(screen.getByText(/Cannot be undone/)).toBeVisible();
  });

  it("copies the conversion and hands Review and save to the entry form", async () => {
    const user = userEvent.setup();
    const { onCopy, onReview } = setup();

    await convertTo(user, "fish");
    await screen.findByText("set -x API_HOST example.com");

    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopy).toHaveBeenCalledWith("set -x API_HOST example.com");

    await user.click(screen.getByRole("button", { name: "Review and save" }));
    expect(onReview).toHaveBeenCalledWith(conversion().converted);
  });

  /// Saving a conversion creates a new entry, and the panel says so.
  it("never runs or saves a conversion on its own", async () => {
    const user = userEvent.setup();
    setup();

    await convertTo(user, "fish");
    await screen.findByText("set -x API_HOST example.com");

    expect(screen.queryByRole("button", { name: /^Run/ })).toBeNull();
    expect(screen.getByText(/The original entry is untouched/)).toBeVisible();
  });

  it("goes back to the picker to try another shell", async () => {
    const user = userEvent.setup();
    setup();

    await convertTo(user, "fish");
    await screen.findByText("set -x API_HOST example.com");

    await user.click(screen.getByRole("button", { name: /Try another shell/ }));

    expect(screen.queryByText("set -x API_HOST example.com")).toBeNull();
    expect(screen.getByRole("button", { name: "PowerShell" })).toBeVisible();
  });

  it("stops a conversion that is still on the wire", async () => {
    const user = userEvent.setup();
    let reject: (error: unknown) => void = () => undefined;
    vi.mocked(api.convertCommandShell).mockReturnValue(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    setup();

    await convertTo(user, "fish");
    await user.click(await screen.findByRole("button", { name: "Stop" }));
    expect(api.cancelAssistantRequest).toHaveBeenCalledOnce();

    reject({ kind: "ai_cancelled", message: "Request cancelled." });
    expect(await screen.findByText(/Stopped. Nothing was converted./)).toBeVisible();
  });

  it("reports a failure without losing the picker", async () => {
    const user = userEvent.setup();
    vi.mocked(api.convertCommandShell).mockRejectedValue({
      kind: "ai_model",
      message: "That model is not available to this account.",
    });
    setup();

    await convertTo(user, "fish");

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "That model is not available to this account.",
    );
    expect(screen.getByRole("button", { name: "zsh" })).toBeVisible();
  });
});
