import { platform } from "./platform";

/**
 * Copies text to the system clipboard. The platform does the real work: the
 * Tauri plugin on the desktop, the browser clipboard on the web. The fallback
 * covers the desktop case where the plugin's IPC is refused by an older
 * webview.
 */
export async function copyToClipboard(text: string): Promise<void> {
  try {
    await platform.copyToClipboard(text);
  } catch (error) {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      return;
    }
    throw error;
  }
}
