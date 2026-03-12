/**
 * Returns true when the app is running inside a Tauri webview (desktop),
 * false when served as a regular web page.
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Opens a URL externally.
 * - In Tauri: uses the OS default browser via tauri-plugin-opener
 * - In web: uses window.open with noopener/noreferrer for safety
 */
export async function openUrl(url: string): Promise<void> {
	if (isTauri()) {
		const { openUrl: tauriOpenUrl } = await import('@tauri-apps/plugin-opener');
		await tauriOpenUrl(url);
	} else {
		window.open(url, '_blank', 'noopener,noreferrer');
	}
}
