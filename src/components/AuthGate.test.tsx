import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AuthGate } from "./AuthGate";
import { platform } from "../lib/platform";

/**
 * The gate is what stands between a browser with no session and the library.
 * These drive it through a stubbed platform, because the real one under test
 * is the desktop implementation, which has no sign-in at all.
 */

const signedOutHandlers = new Set<() => void>();

const auth = {
  status: vi.fn(),
  signIn: vi.fn(),
  signOut: vi.fn(),
  onSignedOut: (handler: () => void) => {
    signedOutHandlers.add(handler);
    return () => signedOutHandlers.delete(handler);
  },
};

function library() {
  return (
    <AuthGate>
      <p>The library</p>
    </AuthGate>
  );
}

describe("the sign-in gate", () => {
  beforeEach(() => {
    signedOutHandlers.clear();
    auth.status.mockReset().mockResolvedValue(false);
    auth.signIn.mockReset().mockResolvedValue(undefined);
    auth.signOut.mockReset().mockResolvedValue(undefined);
    vi.spyOn(platform, "auth", "get").mockReturnValue(auth);
  });

  it("shows the library straight away where there is nothing to sign in to", () => {
    vi.spyOn(platform, "auth", "get").mockReturnValue(null);

    render(library());

    expect(screen.getByText("The library")).toBeInTheDocument();
  });

  it("asks for a password before showing anything behind it", async () => {
    render(library());

    expect(await screen.findByLabelText("Password")).toBeInTheDocument();
    expect(screen.queryByText("The library")).not.toBeInTheDocument();
    // Nothing behind the password was fetched to draw this screen.
    expect(auth.signIn).not.toHaveBeenCalled();
  });

  it("goes straight to the library when the browser is still signed in", async () => {
    auth.status.mockResolvedValue(true);

    render(library());

    expect(await screen.findByText("The library")).toBeInTheDocument();
  });

  /** A server that is not answering must not look like a signed-in one. */
  it("treats an unreachable server as signed out", async () => {
    auth.status.mockRejectedValue({ kind: "network", message: "Could not reach it" });

    render(library());

    expect(await screen.findByLabelText("Password")).toBeInTheDocument();
  });

  it("opens the library once the password is accepted", async () => {
    const user = userEvent.setup();
    render(library());

    await user.type(await screen.findByLabelText("Password"), "a good long password");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(auth.signIn).toHaveBeenCalledWith("a good long password");
    expect(await screen.findByText("The library")).toBeInTheDocument();
  });

  it("keeps the password screen up and says why when it is refused", async () => {
    const user = userEvent.setup();
    auth.signIn.mockRejectedValue({
      kind: "unauthorized",
      message: "That password was not right.",
    });
    render(library());

    await user.type(await screen.findByLabelText("Password"), "wrong guess");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByText(/that password was not right/i)).toBeInTheDocument();
    expect(screen.queryByText("The library")).not.toBeInTheDocument();
    // Cleared, so the next attempt starts from an empty field rather than a
    // guess the user has to select and delete first.
    expect(screen.getByLabelText("Password")).toHaveValue("");
  });

  it("reports being throttled in the words the server used", async () => {
    const user = userEvent.setup();
    auth.signIn.mockRejectedValue({
      kind: "rate_limited",
      message: "Too many wrong passwords. Try again in 4 seconds.",
    });
    render(library());

    await user.type(await screen.findByLabelText("Password"), "another guess");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByText(/try again in 4 seconds/i)).toBeInTheDocument();
  });

  /** The server restarted, or thirty days ran out. Either way, back to the door. */
  it("closes the library when a session ends without being asked to", async () => {
    auth.status.mockResolvedValue(true);
    render(library());
    expect(await screen.findByText("The library")).toBeInTheDocument();

    act(() => {
      for (const handler of signedOutHandlers) handler();
    });

    await waitFor(() => expect(screen.queryByText("The library")).not.toBeInTheDocument());
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });
});
