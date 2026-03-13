import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import health from './routes/health.js';
import settings from './routes/settings.js';
import profiles from './routes/profiles.js';
import likedArtists from './routes/liked-artists.js';
import stats from './routes/stats.js';
import lastfm from './routes/lastfm.js';
import subsonic from './routes/subsonic.js';
import lyrics from './routes/lyrics.js';

const app = new Hono();

app.use('*', cors({ origin: '*', allowMethods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'] }));

const api = app.basePath('/api');
api.route('/health', health);
api.route('/settings', settings);
api.route('/profiles', profiles);
api.route('/liked-artists', likedArtists);
api.route('/stats', stats);
api.route('/lastfm', lastfm);
api.route('/subsonic', subsonic);
api.route('/lyrics', lyrics);

const port = Number(process.env.PORT ?? 8787);

serve({ fetch: app.fetch, port }, () => {
  console.log(`api listening on http://localhost:${port}/api`);
});
