import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";

import { useAppHotkeys } from "./useAppHotkeys";

function Harness({
  aiReady,
  onToggleAssistant,
}: {
  aiReady: boolean;
  onToggleAssistant: () => void;
}) {
  useAppHotkeys({
    view: "library",
    aiReady,
    search: "",
    searchRef: createRef(),
    onCreate: vi.fn(),
    onOpenPalette: vi.fn(),
    onToggleAssistant,
    onOpenShortcuts: vi.fn(),
    onClearSearch: vi.fn(),
    onDismissNavigation: vi.fn(),
  });
  return <textarea aria-label="Composer" />;
}

describe("useAppHotkeys", () => {
  it("opens the assistant while typing only when AI is ready", async () => {
    const user = userEvent.setup();
    const onToggleAssistant = vi.fn();
    const { rerender } = render(
      <Harness aiReady={false} onToggleAssistant={onToggleAssistant} />,
    );

    await user.click(screen.getByLabelText("Composer"));
    await user.keyboard("{Control>}{Shift>}k{/Shift}{/Control}");
    expect(onToggleAssistant).not.toHaveBeenCalled();

    rerender(<Harness aiReady onToggleAssistant={onToggleAssistant} />);
    await user.keyboard("{Control>}{Shift>}k{/Shift}{/Control}");
    expect(onToggleAssistant).toHaveBeenCalledOnce();
  });
});
