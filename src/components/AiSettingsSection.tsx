import { useEffect, useMemo, useState } from "react";

import { api, toAppError } from "../lib/ipc";
import type { AiStatus, AppSettings } from "../lib/types";
import { Button } from "./ui/Button";
import { CheckboxField, SelectField, TextField } from "./ui/Field";

const CUSTOM_MODEL_VALUE = "__custom__";

interface AiSettingsSectionProps {
  draft: AppSettings;
  saved: AppSettings;
  onPatch: (changes: Partial<AppSettings>) => void;
}

interface Notice {
  tone: "ok" | "error" | "info";
  message: string;
}

export function AiSettingsSection({ draft, saved, onPatch }: AiSettingsSectionProps) {
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [customSelected, setCustomSelected] = useState(false);
  const [keyFormOpen, setKeyFormOpen] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [keyAction, setKeyAction] = useState<"save" | "remove" | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [testing, setTesting] = useState(false);
  const [clearingCache, setClearingCache] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);

  useEffect(() => {
    if (!draft.aiEnabled) {
      setLoadingStatus(false);
      return;
    }

    let active = true;
    setLoadingStatus(true);
    api
      .getAiStatus()
      .then((status) => {
        if (!active) return;
        setAiStatus(status);
        if (draft.aiModel && !status.models.includes(draft.aiModel)) {
          setCustomSelected(true);
        }
      })
      .catch((caught) => {
        if (active) {
          setNotice({ tone: "error", message: toAppError(caught).message });
        }
      })
      .finally(() => {
        if (active) setLoadingStatus(false);
      });
    return () => {
      active = false;
    };
  }, [draft.aiEnabled]);

  useEffect(() => {
    if (draft.aiModel === null || aiStatus?.models.includes(draft.aiModel)) {
      setCustomSelected(false);
    } else if (draft.aiModel && aiStatus) {
      setCustomSelected(true);
    }
  }, [aiStatus, draft.aiModel]);

  const modelOptions = useMemo(
    () => [
      {
        value: "",
        label: aiStatus
          ? `Default (${aiStatus.defaultModel})`
          : "Loading available models…",
      },
      ...(aiStatus?.models.map((model) => ({ value: model, label: model })) ?? []),
      { value: CUSTOM_MODEL_VALUE, label: "Custom model…" },
    ],
    [aiStatus],
  );

  const selectedModel = customSelected ? CUSTOM_MODEL_VALUE : (draft.aiModel ?? "");
  const modelSaved =
    (saved.aiModel?.trim() || null) === (draft.aiModel?.trim() || null);
  const customModelError =
    customSelected && !draft.aiModel?.trim()
      ? "Enter a model ID or choose a model from the list."
      : customSelected && /\s/.test(draft.aiModel ?? "")
        ? "Model IDs cannot contain spaces."
        : undefined;
  const readyToTest =
    saved.aiEnabled &&
    draft.aiEnabled &&
    modelSaved &&
    aiStatus?.keyStored === true &&
    aiStatus.credentialManagerAvailable;

  const updateModel = (value: string) => {
    setNotice(null);
    if (value === CUSTOM_MODEL_VALUE) {
      setCustomSelected(true);
      if (!draft.aiModel || aiStatus?.models.includes(draft.aiModel)) {
        onPatch({ aiModel: "" });
      }
      return;
    }
    setCustomSelected(false);
    onPatch({ aiModel: value || null });
  };

  const saveKey = async () => {
    setKeyAction("save");
    setNotice(null);
    try {
      const result = await api.saveAiKey(apiKey);
      setAiStatus((current) =>
        current
          ? {
              ...current,
              keyStored: result.keyStored,
              credentialManagerAvailable: true,
            }
          : current,
      );
      setApiKey("");
      setKeyFormOpen(false);
      setNotice({ tone: "ok", message: "OpenAI API key stored securely" });
    } catch (caught) {
      setNotice({ tone: "error", message: toAppError(caught).message });
    } finally {
      setKeyAction(null);
    }
  };

  const removeKey = async () => {
    setKeyAction("remove");
    setNotice(null);
    try {
      const result = await api.removeAiKey();
      setAiStatus((current) =>
        current ? { ...current, keyStored: result.keyStored } : current,
      );
      setConfirmRemove(false);
      setKeyFormOpen(false);
      setApiKey("");
      setNotice({ tone: "ok", message: "Stored OpenAI API key removed" });
    } catch (caught) {
      setNotice({ tone: "error", message: toAppError(caught).message });
    } finally {
      setKeyAction(null);
    }
  };

  const testConnection = async () => {
    setTesting(true);
    setNotice({ tone: "info", message: "Contacting OpenAI…" });
    try {
      const result = await api.testAiConnection();
      setNotice({
        tone: "ok",
        message: `Connected with ${result.model} using Responses and structured output`,
      });
    } catch (caught) {
      setNotice({ tone: "error", message: toAppError(caught).message });
    } finally {
      setTesting(false);
    }
  };

  const clearExplanations = async () => {
    setClearingCache(true);
    setNotice(null);
    try {
      const removed = await api.clearAiExplanations();
      setNotice({
        tone: "ok",
        message:
          removed === 0
            ? "There were no saved explanations to remove"
            : `Removed ${removed} saved ${removed === 1 ? "explanation" : "explanations"}`,
      });
    } catch (caught) {
      setNotice({ tone: "error", message: toAppError(caught).message });
    } finally {
      setClearingCache(false);
    }
  };

  return (
    <section className="settings__section">
      <h2>AI</h2>
      <CheckboxField
        label="Enable AI features"
        hint="Off by default. When off, AI entry points stay hidden and Rust blocks AI requests."
        checked={draft.aiEnabled}
        onChange={(event) => {
          onPatch({ aiEnabled: event.target.checked });
          setNotice(null);
        }}
      />

      {draft.aiEnabled && (
        <div className="ai-settings">
          <p className="settings__description">
            AI requests go directly from Rust to OpenAI. The API key is stored by your operating
            system, never in the library database.
          </p>

          <SelectField
            label="OpenAI model"
            value={selectedModel}
            onChange={(event) => updateModel(event.target.value)}
            options={modelOptions}
            disabled={loadingStatus || !aiStatus}
            hint="Choose a current model or enter any compatible model ID."
          />

          {customSelected && (
            <TextField
              label="Custom model ID"
              value={draft.aiModel ?? ""}
              maxLength={256}
              spellCheck={false}
              autoCapitalize="none"
              autoCorrect="off"
              placeholder="gpt-5 or a fine-tuned model ID"
              onChange={(event) => onPatch({ aiModel: event.target.value })}
              error={customModelError}
            />
          )}

          <div className="ai-settings__credential">
            <div>
              <span className="field__label">OpenAI API key</span>
              <p className="ai-settings__key-state">
                {loadingStatus
                  ? "Checking the credential manager…"
                  : !aiStatus?.credentialManagerAvailable
                    ? "Secure storage not verified"
                    : aiStatus.keyStored
                      ? "Key stored securely"
                      : "No key stored"}
              </p>
            </div>

            {!loadingStatus && (
              <div className="settings__button-row">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    setKeyFormOpen((open) => !open);
                    setConfirmRemove(false);
                    setNotice(null);
                  }}
                >
                  {aiStatus?.keyStored ? "Replace key" : "Add key"}
                </Button>
                {aiStatus?.keyStored && (
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => {
                      setConfirmRemove(true);
                      setKeyFormOpen(false);
                      setNotice(null);
                    }}
                  >
                    Remove key
                  </Button>
                )}
              </div>
            )}
          </div>



          {keyFormOpen && (
            <div className="ai-settings__key-form">
              <TextField
                label={aiStatus?.keyStored ? "Replacement API key" : "OpenAI API key"}
                type="password"
                value={apiKey}
                autoComplete="new-password"
                spellCheck={false}
                autoCapitalize="none"
                autoCorrect="off"
                placeholder="Paste the key once"
                onChange={(event) => setApiKey(event.target.value)}
                hint="The field is cleared as soon as the operating system stores the key."
              />
              <div className="settings__button-row">
                <Button
                  variant="primary"
                  size="sm"
                  loading={keyAction === "save"}
                  disabled={!apiKey.trim()}
                  onClick={() => void saveKey()}
                >
                  Store key
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setKeyFormOpen(false);
                    setApiKey("");
                  }}
                >
                  Cancel
                </Button>
              </div>
            </div>
          )}

          {confirmRemove && (
            <div className="ai-settings__remove-confirm" role="group" aria-label="Remove API key">
              <p>Remove the stored key from the operating system credential manager?</p>
              <div className="settings__button-row">
                <Button
                  variant="danger"
                  size="sm"
                  loading={keyAction === "remove"}
                  onClick={() => void removeKey()}
                >
                  Remove stored key
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setConfirmRemove(false)}>
                  Cancel
                </Button>
              </div>
            </div>
          )}

          <div className="ai-settings__test">
            <Button
              variant="secondary"
              onClick={() => void testConnection()}
              loading={testing}
              disabled={!readyToTest}
              title={
                readyToTest
                  ? "Send a small structured request to OpenAI"
                  : "Store a key and save the enabled AI settings before testing"
              }
            >
              Test connection
            </Button>
          </div>
        </div>
      )}

      {/* Outside the enabled block on purpose: turning AI off hides saved
          explanations, it does not delete them, so removing them has to stay
          possible either way. */}
      <div className="ai-settings__cache">
        <div>
          <span className="field__label">Saved explanations</span>
          <p className="field__hint">
            Explanations are stored in the local library database so they load without another
            request. Removing them frees that space and takes their text out of search.
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          loading={clearingCache}
          onClick={() => void clearExplanations()}
        >
          Clear saved explanations
        </Button>
      </div>

      {notice && (
        <p
          className={[
            "settings__status",
            notice.tone === "error" ? "is-error" : "",
            notice.tone === "info" ? "is-info" : "",
          ]
            .filter(Boolean)
            .join(" ")}
          role={notice.tone === "error" ? "alert" : "status"}
        >
          {notice.message}
        </p>
      )}
    </section>
  );
}
