import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { api, SETTINGS_CHANGED, toAppError } from "../lib/ipc";
import type { AppSettings, ThemePreference } from "../lib/types";

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "dark",
  commandViewMode: "compact",
  confirmBeforeDelete: true,
};

export interface SettingsState {
  settings: AppSettings;
  loading: boolean;
  error: string | null;
  save: (next: AppSettings) => Promise<AppSettings>;
}

export function useSettings(): SettingsState {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    api
      .getSettings()
      .then((loaded) => {
        if (active) setSettings(loaded);
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

  // Keep the current window in sync with settings saved elsewhere.
  useEffect(() => {
    const unlisten = listen<AppSettings>(SETTINGS_CHANGED, (event) => {
      setSettings(event.payload);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const save = useCallback(async (next: AppSettings) => {
    const saved = await api.saveSettings(next);
    setSettings(saved);
    return saved;
  }, []);

  return { settings, loading, error, save };
}

/** Applies the theme preference to the document, following the OS when asked. */
export function useTheme(preference: ThemePreference): void {
  useEffect(() => {
    const root = document.documentElement;

    if (preference !== "system") {
      root.dataset.theme = preference;
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => {
      root.dataset.theme = media.matches ? "light" : "dark";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [preference]);
}
