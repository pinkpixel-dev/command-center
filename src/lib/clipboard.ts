import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/**
 * Copies text to the system clipboard. The Tauri plugin is the real path; the
 * browser API is the fallback for the rare case where the plugin is
 * unavailable (older webviews refusing the IPC).
 */
export async function copyToClipboard(text: string): Promise<void> {
  try {
    await writeText(text);
  } catch (error) {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      return;
    }
    throw error;
  }
}
