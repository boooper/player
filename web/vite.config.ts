import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  // Tauri: prevent Vite from clearing the terminal
  clearScreen: false,
  server: {
    host: host || '0.0.0.0',
    port: 5173,
    strictPort: true,
    // Tauri uses a fixed port and requires HMR to work over the same host
    hmr: { protocol: 'ws', host: host || 'localhost', port: 5174 },
    // Don't watch Rust source files — let Tauri handle that
    watch: { ignored: ['**/src-tauri/**'] }
  }
});
