import type { PageServerLoad } from './$types';
import { getSettings } from '$lib/server/settings';
import { prisma } from '$lib/server/prisma';
import { isConnected, getStoredUsername } from '$lib/server/lastfm';

export const load: PageServerLoad = async () => {
  const [settingsValues, profiles, connected, username] = await Promise.all([
    getSettings(['LASTFM_API_KEY', 'RECOMMENDATION_PROVIDER', 'METADATA_PROVIDER', 'LASTFM_SHARED_SECRET']),
    prisma.subsonicProfile.findMany({ orderBy: { createdAt: 'asc' } }),
    isConnected(),
    getStoredUsername()
  ]);

  return {
    settings: {
      lastFmApiKey: settingsValues.LASTFM_API_KEY,
      lastFmSharedSecretConfigured: Boolean(settingsValues.LASTFM_SHARED_SECRET),
      recommendationProvider: settingsValues.RECOMMENDATION_PROVIDER || 'lastfm',
      metadataProvider: settingsValues.METADATA_PROVIDER || 'both'
    },
    lastFm: {
      connected,
      username
    },
    // Strip passwords before sending to client
    profiles: profiles.map(({ password: _pw, ...p }) => p)
  };
};
