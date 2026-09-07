import { useCallback, useEffect, useState } from "react";

import { toAppError } from "../lib/ipc";
import { platform } from "../lib/platform";

/**
 * Whether the app is allowed to show the library yet.
 *
 * "notRequired" is the desktop app, where there is nothing to sign in to. The
 * other three are the self-hosted server: asking, in, and out.
 */
export type SessionStatus = "checking" | "signedIn" | "signedOut" | "notRequired";

export interface SessionState {
  status: SessionStatus;
  /** Rejects with a readable message, which the sign-in form shows. */
  signIn: (password: string) => Promise<void>;
  signOut: () => Promise<void>;
}

export function useSession(): SessionState {
  const auth = platform.auth;
  const [status, setStatus] = useState<SessionStatus>(auth ? "checking" : "notRequired");

  // One question on load: is this browser still signed in? A server that
  // cannot be reached is treated as signed out, because the sign-in screen is
  // where the retry lives.
  useEffect(() => {
    if (!auth) return;

    let active = true;
    auth
      .status()
      .then((signedIn) => {
        if (active) setStatus(signedIn ? "signedIn" : "signedOut");
      })
      .catch(() => {
        if (active) setStatus("signedOut");
      });

    return () => {
      active = false;
    };
  }, [auth]);

  // A session can end without anybody asking: the server restarts, or the
  // thirty days run out. Whichever request finds out says so here.
  useEffect(() => auth?.onSignedOut(() => setStatus("signedOut")), [auth]);

  const signIn = useCallback(
    async (password: string) => {
      if (!auth) return;
      try {
        await auth.signIn(password);
      } catch (caught) {
        throw new Error(toAppError(caught).message);
      }
      setStatus("signedIn");
    },
    [auth],
  );

  const signOut = useCallback(async () => {
    if (!auth) return;
    // Signed out here whatever the server said. A failed request must not
    // leave someone looking at a library they asked to close.
    try {
      await auth.signOut();
    } finally {
      setStatus("signedOut");
    }
  }, [auth]);

  return { status, signIn, signOut };
}
