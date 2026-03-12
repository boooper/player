import type { LayoutServerLoad } from './$types';
import { getSettings } from '$lib/server/settings';

export const load: LayoutServerLoad = async () => {
  const settings = await getSettings(['LASTFM_API_KEY', 'RECOMMENDATION_PROVIDER', 'METADATA_PROVIDER']);
  return {
    lastFmApiKey: settings.LASTFM_API_KEY,
    recommendationProvider: settings.RECOMMENDATION_PROVIDER || 'lastfm',
    metadataProvider: settings.METADATA_PROVIDER || 'both'
  };
};
