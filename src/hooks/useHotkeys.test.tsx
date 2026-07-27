import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { useHotkeys } from "./useHotkeys";
import type { Hotkey } from "./useHotkeys";

function Harness({ hotkeys }: { hotkeys: Hotkey[] }) {
  useHotkeys(hotkeys);
  return <textarea aria-label="composer" />;
}

describe("useHotkeys", () => {
  it("keeps a character shortcut out of whatever the user is typing into", async () => {
    const user = userEvent.setup();
    const openShortcuts = vi.fn();
    render(<Harness hotkeys={[{ combo: "shift+?", handler: openShortcuts }]} />);

    const composer = screen.getByLabelText("composer");
    await user.click(composer);
    await user.type(composer, "what does this do?");

    expect(openShortcuts).not.toHaveBeenCalled();
    expect(composer).toHaveValue("what does this do?");

    // Still available from anywhere that is not a field. A real `?` arrives
    // with Shift held, which is what the combo is written for.
    await user.click(document.body);
    await user.keyboard("{Shift>}?{/Shift}");
    expect(openShortcuts).toHaveBeenCalledOnce();
  });

  it("lets an opted-in shortcut through while typing", async () => {
    const user = userEvent.setup();
    const openPalette = vi.fn();
    render(
      <Harness hotkeys={[{ combo: "mod+k", allowWhileTyping: true, handler: openPalette }]} />,
    );

    await user.click(screen.getByLabelText("composer"));
    await user.keyboard("{Control>}k{/Control}");

    expect(openPalette).toHaveBeenCalledOnce();
  });
});
