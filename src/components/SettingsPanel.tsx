import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { APP_NAME, APP_VERSION, MAKER, MAKER_URL, GITHUB_URL, SUPPORT_EMAIL } from "../lib/app-info";
import { api, toAppError } from "../lib/ipc";
import type {
  AppSettings,
  CommandViewMode,
  ThemePreference,
} from "../lib/types";
import { Button } from "./ui/Button";
import { CheckboxField, SelectField } from "./ui/Field";

export interface SettingsPanelProps {
  settings: AppSettings;
  onSave: (next: AppSettings) => Promise<AppSettings>;
}

export function SettingsPanel({ settings, onSave }: SettingsPanelProps) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [status, setStatus] = useState<{ tone: "ok" | "error"; message: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const [location, setLocation] = useState("");

  useEffect(() => setDraft(settings), [settings]);

  useEffect(() => {
    api
      .libraryLocation()
      .then(setLocation)
      .catch(() => setLocation(""));
  }, []);

  const patch = (changes: Partial<AppSettings>) =>
    setDraft((current) => ({ ...current, ...changes }));

  const save = async () => {
    setSaving(true);
    setStatus(null);
    try {
      await onSave(draft);
      setStatus({ tone: "ok", message: "Settings saved" });
    } catch (caught) {
      setStatus({ tone: "error", message: toAppError(caught).message });
      setDraft(settings);
    } finally {
      setSaving(false);
    }
  };

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  return (
    <div className="settings">
      <section className="settings__section">
        <h2>General</h2>

        <SelectField
          label="Theme"
          value={draft.theme}
          onChange={(event) => patch({ theme: event.target.value as ThemePreference })}
          options={[
            { value: "dark", label: "Dark" },
            { value: "high-contrast", label: "High contrast dark" },
            { value: "light", label: "Light" },
            { value: "system", label: "Match the system" },
          ]}
        />

        <CheckboxField
          label="Ask before deleting a command"
          checked={draft.confirmBeforeDelete}
          onChange={(event) => patch({ confirmBeforeDelete: event.target.checked })}
        />
      </section>

      <section className="settings__section">
        <h2>View</h2>

        <SelectField
          label="Library view"
          value={draft.commandViewMode}
          onChange={(event) =>
            patch({ commandViewMode: event.target.value as CommandViewMode })
          }
          options={[
            { value: "compact", label: "Compact list" },
            { value: "cards", label: "Cards" },
          ]}
          hint="Compact keeps each command easy to scan. Cards use more width when it is available."
        />
      </section>

      <div className="settings__actions">
        <Button variant="primary" onClick={() => void save()} loading={saving} disabled={!dirty}>
          Save settings
        </Button>
        {dirty && !saving && <span className="field__hint">Unsaved changes</span>}
        {status && (
          <span
            className={status.tone === "ok" ? "settings__status" : "settings__status is-error"}
            role="status"
          >
            {status.message}
          </span>
        )}
      </div>

      <section className="settings__section">
        <h2>About</h2>
        <dl className="card__facts">
          <div>
            <dt>App</dt>
            <dd>
              {APP_NAME} v{APP_VERSION}
            </dd>
          </div>

          <div>
            <dt>Made by</dt>
            <dd>
              <button
                type="button"
                className="link-button"
                onClick={() => void openUrl(MAKER_URL)}
              >
                {MAKER}
              </button>
            </dd>
          </div>
          <div>
            <dt>Support</dt>
            <dd>
              <button
                type="button"
                className="link-button"
                onClick={() => void openUrl(SUPPORT_EMAIL)}
              >
                {SUPPORT_EMAIL}
              </button>
            </dd>
          </div>
          <div>
            <dt>GitHub</dt>
            <dd>
              <button
                type="button"
                className="link-button"
                onClick={() => void openUrl(GITHUB_URL)}
              >
                {GITHUB_URL}
              </button>
            </dd>
          </div>
                {location && (
            <div>
              <dt>Library file</dt>
              <dd className="mono">{location}</dd>
            </div>
          )}
        </dl>
      </section>
    </div>
  );
}
