import { env } from '$env/dynamic/public';
import { isTauri } from './tauri';

export function apiUrl(path: string): string {
  const configuredBase = (env.PUBLIC_API_BASE_URL ?? '').replace(/\/+$/, '');
  const apiBaseUrl = configuredBase || (isTauri() ? 'http://127.0.0.1:8787' : '');
  if (/^https?:\/\//i.test(path)) return path;
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  return apiBaseUrl ? `${apiBaseUrl}${normalizedPath}` : normalizedPath;
}

export function apiFetch(path: string, init?: RequestInit): Promise<Response> {
  return fetch(apiUrl(path), init);
}
