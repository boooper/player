import 'dotenv/config';
import { serve } from '@hono/node-server';
import app from './app.js';

const port = Number(process.env.PORT ?? 8787);

const server = serve({ fetch: app.fetch, port }, () => {
  // Tauri captures stdout — this signals the sidecar is ready.
  console.log(`[api] listening on http://localhost:${port}/api`);
});

// Graceful shutdown: Tauri sends SIGTERM when the window is destroyed.
function shutdown(signal: string) {
  console.log(`[api] received ${signal}, shutting down`);
  server.close(() => {
    console.log('[api] server closed');
    process.exit(0);
  });
}

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));
