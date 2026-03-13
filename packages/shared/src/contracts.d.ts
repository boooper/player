export declare const SERVICE_STATUS: {
  readonly CHECKING: 'checking';
  readonly ONLINE: 'online';
  readonly OFFLINE: 'offline';
  readonly MISSING: 'missing';
};

export type ServiceStatus = (typeof SERVICE_STATUS)[keyof typeof SERVICE_STATUS];

export declare const RECOMMENDATION_PROVIDERS: {
  readonly LASTFM: 'lastfm';
};

export type RecommendationProvider =
  (typeof RECOMMENDATION_PROVIDERS)[keyof typeof RECOMMENDATION_PROVIDERS];

export declare const METADATA_PROVIDERS: {
  readonly BOTH: 'both';
  readonly LASTFM: 'lastfm';
  readonly AUDIODB: 'audiodb';
};

export type MetadataProvider =
  (typeof METADATA_PROVIDERS)[keyof typeof METADATA_PROVIDERS];
