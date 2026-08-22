import { useCallback, useEffect, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import type { AppSettings, CodexAvailability, CodexStatus, CodexUnusableReason } from "../lib/types";
import { Button } from "./ui/Button";
import { TextField } from "./ui/Field";

interface CodexPanelProps {
  draft: AppSettings;
  saved: AppSettings;
  onPatch: (changes: Partial<AppSettings>) => void;
}

const INSTALL_COMMAND = "npm install -g @openai/codex";

/** What the panel says about the Codex program itself. */
function availabilityHeadline(availability: CodexAvailability): string {
  switch (availability.state) {
    case "ready":
      return `Codex ${availability.version} found`;
    case "notFound":
      return "Codex was not found";
    case "tooOld":
      return `Codex ${availability.version} is too old`;
    case "unusable":
      return "Codex could not be used";
    case "unsupportedPlatform":
      return "Codex is not available on this platform";
  }
}

function availabilityDetail(availability: CodexAvailability): string {
  switch (availability.state) {
    case "ready":
      return "Command Center keeps its own Codex settings, separate from your Codex CLI.";
    case "notFound":
      return `Install Codex, then choose Check again. If it is already installed somewhere unusual, enter its full path below.`;
    case "tooOld":
      return `This feature needs ${availability.minimum} or newer. Update Codex, then choose Check again.`;
    case "unusable":
      return unusableDetail(availability.reason);
    case "unsupportedPlatform":
      return "This build cannot start Codex. The OpenAI API key provider still works here.";
  }
}

function unusableDetail(reason: CodexUnusableReason): string {
  switch (reason) {
    case "missing":
      return "Nothing is at that path. Check the path below, or clear it to search again.";
    case "notExecutable":
      return "That file is not marked as a program you can run.";
    case "shimNotSupported":
      return "That is a launcher script rather than the Codex program. Enter the path to the Codex executable itself.";
    case "noVersion":
      return "That program did not report a Codex version, so it is probably not Codex.";
    case "timeout":
      return "Codex did not respond when asked for its version.";
  }
}

export function CodexPanel({ draft, saved, onPatch }: CodexPanelProps) {
  const [status, setStatus] = useState<CodexStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showDiagnostics, setShowDiagnostics] = useState(false);

  const load = useCallback(async (fresh: boolean) => {
    setError(null);
    try {
      const next = fresh ? await api.refreshCodex() : await api.getCodexStatus();
      setStatus(next);
    } catch (caught) {
      setError(toAppError(caught).message);
      setStatus(null);
    }
  }, []);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void api
      .getCodexStatus()
      .then((next) => {
        if (active) setStatus(next);
      })
      .catch((caught) => {
        if (active) setError(toAppError(caught).message);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const checkAgain = async () => {
    setChecking(true);
    await load(true);
    setChecking(false);
  };

  const pathChanged = (draft.codexPath ?? "") !== (saved.codexPath ?? "");
  const availability = status?.availability;
  const isReady = availability?.state === "ready";

  return (
    <div className="codex-panel">
      <div className="ai-settings__credential">
        <div>
          <span className="field__label">Codex program</span>
          <p className="ai-settings__key-state" role="status">
            {loading
              ? "Looking for Codex…"
              : availability
                ? availabilityHeadline(availability)
                : "Codex status is unavailable"}
          </p>
          {!loading && availability && (
            <p className="codex-panel__detail">{availabilityDetail(availability)}</p>
          )}
        </div>

        {!loading && availability?.state !== "unsupportedPlatform" && (
          <div className="settings__button-row">
            <Button
              variant="secondary"
              size="sm"
              loading={checking}
              onClick={() => void checkAgain()}
            >
              Check again
            </Button>
          </div>
        )}
      </div>

      {!loading && availability?.state === "notFound" && (
        <div className="codex-panel__install">
          <span className="field__label" id="codex-install-command">
            Install with
          </span>
          <code className="codex-panel__command" aria-labelledby="codex-install-command">
            {INSTALL_COMMAND}
          </code>
        </div>
      )}

      {!loading && availability?.state !== "unsupportedPlatform" && (
        <TextField
          label="Codex program path"
          value={draft.codexPath ?? ""}
          spellCheck={false}
          autoCapitalize="none"
          autoCorrect="off"
          maxLength={4096}
          placeholder="Leave empty to search automatically"
          hint={
            pathChanged
              ? "Save settings, then choose Check again to use this path."
              : "Only needed when Codex is installed somewhere unusual."
          }
          onChange={(event) => onPatch({ codexPath: event.target.value.trim() || null })}
        />
      )}

      {isReady && (
        <div className="ai-settings__credential">
          <div>
            <span className="field__label">ChatGPT account</span>
            <p className="ai-settings__key-state" role="status">
              Connecting a ChatGPT account is not available in this version.
            </p>
          </div>
        </div>
      )}

      {status?.accountError && (
        <p className="settings__status is-error" role="alert">
          {status.accountError}
        </p>
      )}

      {error && (
        <p className="settings__status is-error" role="alert">
          {error}
        </p>
      )}

      {status && status.diagnostics.length > 0 && (
        <div className="codex-panel__diagnostics">
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={showDiagnostics}
            onClick={() => setShowDiagnostics((open) => !open)}
          >
            {showDiagnostics ? "Hide details" : "Show details"}
          </Button>
          {showDiagnostics && (
            <ul className="codex-panel__log">
              {status.diagnostics.map((line, index) => (
                <li key={`${index}-${line}`}>{line}</li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
