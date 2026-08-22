import { useCallback, useEffect, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import type {
  AppSettings,
  CodexLoginPrompt,
  CodexModel,
  CodexStatus,
} from "../lib/types";
import { Button } from "./ui/Button";
import { SelectField } from "./ui/Field";

interface CodexAccountSectionProps {
  status: CodexStatus;
  draft: AppSettings;
  saved: AppSettings;
  onPatch: (changes: Partial<AppSettings>) => void;
  onStatus: (status: CodexStatus) => void;
}

type Phase = "idle" | "starting" | "waiting" | "disconnecting";

export function CodexAccountSection({
  status,
  draft,
  saved,
  onPatch,
  onStatus,
}: CodexAccountSectionProps) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [prompt, setPrompt] = useState<CodexLoginPrompt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [models, setModels] = useState<CodexModel[] | null>(null);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [confirmDisconnect, setConfirmDisconnect] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);

  const connected = status.account.state === "connected";

  useEffect(() => {
    if (!connected) {
      setModels(null);
      return;
    }

    let active = true;
    setModelsError(null);
    api
      .listCodexModels()
      .then((result) => {
        if (active) setModels(result);
      })
      .catch((caught) => {
        // An empty catalogue is better than an invented one, so the picker
        // stays empty and says why.
        if (active) {
          setModels([]);
          setModelsError(toAppError(caught).message);
        }
      });
    return () => {
      active = false;
    };
  }, [connected]);

  const connect = useCallback(
    async (useDeviceCode: boolean) => {
      setError(null);
      setPhase("starting");
      try {
        const started = await api.startCodexLogin(useDeviceCode);
        setPrompt(started);
        setPhase("waiting");

        const next = await api.awaitCodexLogin();
        onStatus(next);
        setPrompt(null);
        setPhase("idle");
      } catch (caught) {
        setError(toAppError(caught).message);
        setPrompt(null);
        setPhase("idle");
      }
    },
    [onStatus],
  );

  const cancel = async () => {
    try {
      await api.cancelCodexLogin();
    } catch (caught) {
      setError(toAppError(caught).message);
    }
    setPrompt(null);
    setPhase("idle");
  };

  const disconnect = async () => {
    setPhase("disconnecting");
    setError(null);
    try {
      const next = await api.disconnectCodex();
      onStatus(next);
      // The model belonged to the account that just went away.
      onPatch({ codexModel: null });
      setConfirmDisconnect(false);
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setPhase("idle");
    }
  };

  const test = async () => {
    setTesting(true);
    setTestResult(null);
    setError(null);
    try {
      const result = await api.testAiConnection();
      setTestResult(`Connected with ${result.model}`);
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setTesting(false);
    }
  };

  const busy = phase !== "idle";
  // Rust reads saved settings, so an unsaved provider or model would test
  // something other than what is on screen.
  const readyToTest =
    connected &&
    saved.aiEnabled &&
    saved.aiProvider === "chatgptCodex" &&
    saved.codexModel !== null &&
    saved.codexModel === draft.codexModel;
  const savedModelMissing =
    connected &&
    models !== null &&
    draft.codexModel !== null &&
    !models.some((model) => model.id === draft.codexModel);

  return (
    <div className="codex-account">
      <div className="ai-settings__credential">
        <div>
          <span className="field__label">ChatGPT account</span>
          <p className="ai-settings__key-state" role="status">
            {accountSummary(status, phase)}
          </p>
        </div>

        {!busy && (
          <div className="settings__button-row">
            {status.account.state === "notConnected" ? (
              <>
                <Button variant="primary" size="sm" onClick={() => void connect(false)}>
                  Connect ChatGPT
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  title="Use this when the browser cannot open or finish the sign-in"
                  onClick={() => void connect(true)}
                >
                  Use a sign-in code
                </Button>
              </>
            ) : (
              <Button
                variant="danger"
                size="sm"
                onClick={() => {
                  setConfirmDisconnect(true);
                  setError(null);
                }}
              >
                Disconnect
              </Button>
            )}
          </div>
        )}

        {phase === "waiting" && (
          <div className="settings__button-row">
            <Button variant="ghost" size="sm" onClick={() => void cancel()}>
              Cancel sign-in
            </Button>
          </div>
        )}
      </div>

      {phase === "waiting" && prompt?.mode === "deviceCode" && (
        <div className="codex-account__device" role="group" aria-label="Sign-in code">
          <p className="codex-panel__detail">
            Open this address on any device and enter the code.
          </p>
          <code className="codex-panel__command">{prompt.verificationUrl}</code>
          <code className="codex-panel__code">{prompt.userCode}</code>
        </div>
      )}

      {confirmDisconnect && (
        <div
          className="ai-settings__remove-confirm"
          role="group"
          aria-label="Disconnect ChatGPT"
        >
          <p>
            Disconnect this ChatGPT account from Command Center on this computer? Your
            Codex CLI sign-in is separate and stays as it is.
          </p>
          <div className="settings__button-row">
            <Button
              variant="danger"
              size="sm"
              loading={phase === "disconnecting"}
              onClick={() => void disconnect()}
            >
              Disconnect
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setConfirmDisconnect(false)}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}

      {connected && (
        <SelectField
          label="Codex model"
          value={draft.codexModel ?? ""}
          disabled={models === null}
          options={[
            {
              value: "",
              label:
                models === null
                  ? "Loading available models…"
                  : models.length === 0
                    ? "No models available"
                    : "Choose a model",
            },
            ...(models ?? []).map((model) => ({
              value: model.id,
              label: model.isDefault
                ? `${model.displayName} (default)`
                : model.displayName,
            })),
          ]}
          error={
            savedModelMissing
              ? "That model is no longer available on this account. Choose another."
              : modelsError ?? undefined
          }
          hint={
            draft.codexModel === null && models !== null && models.length > 0
              ? "Choose a model and save to finish setting up this provider."
              : undefined
          }
          onChange={(event) => onPatch({ codexModel: event.target.value || null })}
        />
      )}

      {connected && (
        <div className="ai-settings__test">
          <Button
            variant="secondary"
            onClick={() => void test()}
            loading={testing}
            disabled={!readyToTest}
            title={
              readyToTest
                ? "Send one small structured request through Codex"
                : "Choose a Codex model and save the settings before testing"
            }
          >
            Test connection
          </Button>
        </div>
      )}

      {readyToTest && (
        <p className="codex-panel__detail" role="status">
          Test connection runs through Codex. The other AI actions still use the
          OpenAI API key provider in this version.
        </p>
      )}

      {testResult && (
        <p className="settings__status" role="status">
          {testResult}
        </p>
      )}

      {error && (
        <p className="settings__status is-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function accountSummary(status: CodexStatus, phase: Phase): string {
  if (phase === "starting") return "Opening ChatGPT sign-in…";
  if (phase === "waiting") return "Waiting for sign-in to finish…";
  if (phase === "disconnecting") return "Disconnecting…";

  switch (status.account.state) {
    case "notConnected":
      return "Not connected";
    case "connected":
      return [status.account.email, status.account.plan]
        .filter(Boolean)
        .join(" · ") || "Connected";
    case "connectedWithOtherCredentials":
      return `Signed in with ${status.account.kind} credentials rather than a ChatGPT account`;
  }
}
