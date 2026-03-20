import type { Window } from '@tauri-apps/api/window';

/**
 * Returns true when the app is running inside a Tauri webview (desktop),
 * false when served as a regular web page.
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// Cached window reference — set once on init, used synchronously after that.
let _win: Window | null = null;

/**
 * Call once at app startup (inside onMount) when running in Tauri.
 * After this resolves, startDragging / minimizeWindow etc. are synchronous.
 */
export async function initTauriWindow(): Promise<void> {
	if (!isTauri()) return;
	const { getCurrentWindow } = await import('@tauri-apps/api/window');
	_win = getCurrentWindow();
}

// Synchronous helpers — all no-ops until initTauriWindow() resolves.
export function startDragging()        { _win?.startDragging(); }
export function minimizeWindow()       { _win?.minimize(); }
export function toggleMaximizeWindow() { _win?.toggleMaximize(); }
export function closeWindow()          { _win?.close(); }

/**
 * Returns current maximized state and subscribes to changes.
 * Call after initTauriWindow(). Returns an unsubscribe function.
 */
export async function watchMaximized(cb: (maximized: boolean) => void): Promise<() => void> {
	if (!_win) return () => {};
	cb(await _win.isMaximized());
	const unlisten = await _win.onResized(async () => {
		cb(await _win!.isMaximized());
	});
	return unlisten;
}

/**
 * Detects the OS platform synchronously from the user agent.
 * Returns 'macos', 'windows', or 'linux'.
 */
export function getOsPlatform(): string {
	const ua = navigator.userAgent;
	if (ua.includes('Mac OS X') || ua.includes('Macintosh')) return 'macos';
	if (ua.includes('Windows')) return 'windows';
	return 'linux';
}

export { Position } from '@tauri-apps/plugin-positioner';

/**
 * Moves the window to a well-known position.
 * No-op when running in a browser.
 */
export async function moveWindow(position: import('@tauri-apps/plugin-positioner').Position): Promise<void> {
	if (!isTauri()) return;
	const { moveWindow: _moveWindow } = await import('@tauri-apps/plugin-positioner');
	await _moveWindow(position);
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
