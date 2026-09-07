import type { ReactNode } from "react";

import { useSession } from "../hooks/useSession";
import { SignIn } from "./SignIn";

export interface AuthGateProps {
  children: ReactNode;
}

/**
 * Decides whether the app runs at all.
 *
 * On the desktop it always does, and this costs one render. On the server the
 * library is not mounted until there is a session, so nothing behind the
 * password is ever fetched, and losing the session unmounts it again.
 */
export function AuthGate({ children }: AuthGateProps) {
  const { status, signIn } = useSession();

  if (status === "checking") {
    // Deliberately bare. This is one request long, and a skeleton of a
    // library nobody is signed in to would be a lie.
    return <div className="signin" role="status" aria-live="polite" aria-busy="true" />;
  }

  if (status === "signedOut") {
    return <SignIn onSignIn={signIn} />;
  }

  return <>{children}</>;
}
