import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { APP_NAME, APP_VERSION, MAKER, MAKER_URL } from "../lib/app-info";
import { prettyShortcut } from "../lib/format";
import { api, toAppError } from "../lib/ipc";
import type { AppSettings, Collection, ThemePreference } from "../lib/types";
import { Button } from "./ui/Button";
import { CheckboxField, SelectField, TextField } from "./ui/Field";

export interface SettingsPanelProps {
  settings: AppSettings;
  collections: Collection[];
  onSave: (next: AppSettings) => Promise<AppSettings>;
}

export function SettingsPanel({ settings, collections, onSave }: SettingsPanelProps) {
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
            { value: "light", label: "Light" },
            { value: "system", label: "Match the system" },
          ]}
        />

        <SelectField
          label="Default collection for new commands"
          value={draft.defaultCollectionId === null ? "" : String(draft.defaultCollectionId)}
          onChange={(event) =>
            patch({ defaultCollectionId: event.target.value ? Number(event.target.value) : null })
          }
          options={[
            { value: "", label: "None" },
            ...collections.map((collection) => ({
              value: String(collection.id),
              label: collection.name,
            })),
          ]}
        />

        <CheckboxField
          label="Ask before deleting a command"
          checked={draft.confirmBeforeDelete}
          onChange={(event) => patch({ confirmBeforeDelete: event.target.checked })}
        />
      </section>

      <section className="settings__section">
        <h2>Quick Add</h2>

        <TextField
          label="Global shortcut"
          value={draft.quickAddShortcut}
          onChange={(event) => patch({ quickAddShortcut: event.target.value })}
          hint={`Currently ${prettyShortcut(draft.quickAddShortcut)}. Use names like CommandOrControl+Shift+Space.`}
          spellCheck={false}
        />

        <CheckboxField
          label="Close the Quick Add window after saving"
          checked={draft.closeQuickAddAfterSave}
          onChange={(event) => patch({ closeQuickAddAfterSave: event.target.checked })}
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
          {location && (
            <div>
              <dt>Library file</dt>
              <dd className="mono">{location}</dd>
            </div>
          )}
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
        </dl>
        <p className="field__hint">
          Everything stays on this machine. No account, no sync, no network calls.
        </p>
      </section>
    </div>
  );
}
