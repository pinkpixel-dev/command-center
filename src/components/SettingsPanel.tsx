import { useEffect, useState } from "react";
import { APP_NAME, APP_VERSION, MAKER, MAKER_URL, GITHUB_URL, SUPPORT_EMAIL } from "../lib/app-info";
import { api, toAppError } from "../lib/ipc";
import { backupLibraryDatabase, exportLibraryMarkdown } from "../lib/library-files";
import { platform } from "../lib/platform";
import { useSession } from "../hooks/useSession";
import type {
  AppSettings,
  CommandViewMode,
  ThemePreference,
} from "../lib/types";
import { Button } from "./ui/Button";
import { CheckboxField, SelectField } from "./ui/Field";
import { AiSettingsSection } from "./AiSettingsSection";

export interface SettingsPanelProps {
  settings: AppSettings;
  onSave: (next: AppSettings) => Promise<AppSettings>;
}

export function SettingsPanel({ settings, onSave }: SettingsPanelProps) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [status, setStatus] = useState<{ tone: "ok" | "error"; message: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const [fileAction, setFileAction] = useState<"export" | "backup" | null>(null);
  // Only the self-hosted server has a session to end. On the desktop this is
  // "notRequired" and the whole section below is skipped.
  const { status: sessionStatus, signOut } = useSession();
  const [fileStatus, setFileStatus] = useState<{ tone: "ok" | "error"; message: string } | null>(
    null,
  );
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
  const invalidAiModel =
    draft.aiEnabled &&
    draft.aiModel !== null &&
    (!draft.aiModel.trim() || /\s/.test(draft.aiModel));

  const runFileAction = async (action: "export" | "backup") => {
    setFileAction(action);
    setFileStatus(null);
    try {
      const destination =
        action === "export" ? await exportLibraryMarkdown() : await backupLibraryDatabase();
      if (destination) {
        setFileStatus({
          tone: "ok",
          message: action === "export" ? "Markdown export saved" : "Library backup saved",
        });
      }
    } catch (caught) {
      setFileStatus({ tone: "error", message: toAppError(caught).message });
    } finally {
      setFileAction(null);
    }
  };

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

        <CheckboxField
          label="Launch Command Center when you sign in"
          checked={draft.launchAtStartup}
          onChange={(event) => patch({ launchAtStartup: event.target.checked })}
        />

        <CheckboxField
          label="Keep running in the tray when the window closes"
          checked={draft.closeToTray}
          onChange={(event) => patch({ closeToTray: event.target.checked })}
        />
      </section>

      <AiSettingsSection draft={draft} saved={settings} onPatch={patch} />

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
        />
      </section>

      <section className="settings__section">
        <h2>Data</h2>
        <div className="settings__button-row">
          <Button
            variant="secondary"
            onClick={() => void runFileAction("export")}
            loading={fileAction === "export"}
            disabled={fileAction !== null}
          >
            Export Markdown
          </Button>
          <Button
            variant="secondary"
            onClick={() => void runFileAction("backup")}
            loading={fileAction === "backup"}
            disabled={fileAction !== null}
          >
            Back up library
          </Button>
        </div>
        {fileStatus && (
          <span
            className={fileStatus.tone === "ok" ? "settings__status" : "settings__status is-error"}
            role="status"
          >
            {fileStatus.message}
          </span>
        )}
      </section>

      {sessionStatus !== "notRequired" && (
        <section className="settings__section">
          <h2>Session</h2>
          <div className="settings__button-row">
            <Button variant="secondary" onClick={() => void signOut()}>
              Sign out
            </Button>
          </div>
        </section>
      )}

      <div className="settings__actions">
        <Button
          variant="primary"
          onClick={() => void save()}
          loading={saving}
          disabled={!dirty || invalidAiModel}
        >
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
                onClick={() => void platform.openUrl(MAKER_URL)}
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
                onClick={() => void platform.openUrl(SUPPORT_EMAIL)}
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
                onClick={() => void platform.openUrl(GITHUB_URL)}
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
