import { useState } from "react";
import type { FormEvent } from "react";

import { APP_NAME } from "../lib/app-info";
import { Button } from "./ui/Button";
import { TextField } from "./ui/Field";

export interface SignInProps {
  onSignIn: (password: string) => Promise<void>;
}

/**
 * The first screen of the self-hosted server, and the only one anybody sees
 * without a session. The desktop app never renders this.
 */
export function SignIn({ onSignIn }: SignInProps) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (busy || password.length === 0) return;

    setBusy(true);
    setError(null);
    try {
      await onSignIn(password);
      // Nothing on success: the gate above swaps this screen for the library.
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "That did not work.");
      setPassword("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="signin">
      <form className="signin__panel" onSubmit={(event) => void submit(event)}>
        <h1 className="signin__title">{APP_NAME}</h1>

        <TextField
          label="Password"
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          error={error ?? undefined}
          autoComplete="current-password"
          // The only field on the screen, so it takes the caret without
          // anybody reaching for it. Phones included.
          autoFocus
          required
          disabled={busy}
        />

        <Button type="submit" variant="primary" loading={busy} disabled={password.length === 0}>
          Sign in
        </Button>
      </form>
    </main>
  );
}
