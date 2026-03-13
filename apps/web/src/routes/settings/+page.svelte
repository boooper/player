<script lang="ts">
  import { onMount } from 'svelte';
  import { Settings, Save, Loader2 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { appSettings, libraryRefresh } from '$lib/stores/settings';
  import { platform } from '@tauri-apps/plugin-os';
  import {
    fetchAppSettings,
    fetchLastFmStatus,
    fetchProfiles,
    getAutostart,
    updateAppSettings,
    type ProfilePayload,
  } from '$lib/api';

  import ServersSection from './ServersSection.svelte';
  import LastFmSection from './LastFmSection.svelte';
  import ApplicationSection from './ApplicationSection.svelte';
  import DangerZoneSection from './DangerZoneSection.svelte';
  import StatsSection from './StatsSection.svelte';

  // ── State ─────────────────────────────────────────────────────────────────
  let profiles = $state<ProfilePayload[]>([]);
  let lastFmApiKey = $state('');
  let lastFmSharedSecret = $state('');
  let lastFmSharedSecretConfigured = $state(false);
  let lfmConnected = $state(false);
  let lfmUsername = $state('');
  let metadataProvider = $state('both');
  let recommendationProvider = $state('lastfm');
  let isMobile = $state(false);
  let autostartEnabled = $state(false);
  let saving = $state(false);

  // Increment to trigger StatsSection reload
  let statsRefreshKey = $state(0);

  onMount(async () => {
    let currentPlatform = 'unknown';
    try { currentPlatform = await platform(); } catch { /* not in Tauri context */ }
    isMobile = currentPlatform === 'android' || currentPlatform === 'ios';
    await loadInitialSettings();
    if (!isMobile) autostartEnabled = await getAutostart().catch(() => false);
  });

  async function loadInitialSettings() {
    // Load profiles independently so a settings error never hides saved servers.
    const profilesPromise = fetchProfiles()
      .then((p) => { profiles = p; })
      .catch(() => { toast.error('Failed to load servers'); });

    const settingsPromise = (async () => {
      try {
        const [settings, lfmStatus] = await Promise.all([
          fetchAppSettings(),
          fetchLastFmStatus().catch(() => ({ connected: false, username: '' }))
        ]);
        lastFmApiKey = settings.lastFmApiKey;
        lastFmSharedSecretConfigured = settings.lastFmSharedSecretConfigured;
        recommendationProvider = settings.recommendationProvider;
        metadataProvider = settings.metadataProvider;
        lfmConnected = lfmStatus.connected;
        lfmUsername = lfmStatus.username;
        appSettings.set({
          lastFmApiKey,
          recommendationProvider,
          metadataProvider,
          lastFmConnected: lfmConnected,
          lastFmUsername: lfmUsername,
        });
      } catch {
        toast.error('Failed to load settings');
      }
    })();

    await Promise.all([profilesPromise, settingsPromise]);
  }

  async function save() {
    saving = true;
    try {
      await updateAppSettings({
        lastFmApiKey,
        recommendationProvider,
        metadataProvider,
        lastFmSharedSecret,
      });
      appSettings.set({ lastFmApiKey, recommendationProvider, metadataProvider, lastFmConnected: lfmConnected, lastFmUsername: lfmUsername });
      libraryRefresh.update((n) => n + 1);
      if (lastFmSharedSecret.trim()) {
        lastFmSharedSecretConfigured = true;
        lastFmSharedSecret = '';
      }
      toast.success('Settings saved');
      statsRefreshKey += 1;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err) || 'Failed to save settings');
    } finally {
      saving = false;
    }
  }

  function handleCleared() {
    profiles = [];
    lastFmApiKey = '';
    lastFmSharedSecretConfigured = false;
    lfmConnected = false;
    lfmUsername = '';
    recommendationProvider = 'lastfm';
    metadataProvider = 'both';
    statsRefreshKey += 1;
  }

</script>

<div class="mx-auto max-w-3xl space-y-8">
  <!-- Page header -->
  <div class="flex items-center gap-3">
    <Settings class="size-7 shrink-0 text-muted-foreground" />
    <div>
      <h1 class="text-2xl font-bold tracking-tight">Settings</h1>
      <p class="text-sm text-muted-foreground">Configure connections, providers, and view library stats.</p>
    </div>
  </div>

  <!-- Service status + Library stats -->
  <StatsSection refreshKey={statsRefreshKey} />

  <!-- Servers -->
  <ServersSection
    bind:profiles
    onHealthChange={() => { statsRefreshKey += 1; }}
  />

  <!-- Last.fm credentials + Providers -->
  <LastFmSection
    bind:lastFmApiKey
    bind:lastFmSharedSecret
    bind:lastFmSharedSecretConfigured
    bind:lfmConnected
    bind:lfmUsername
    bind:metadataProvider
    bind:recommendationProvider
    onHealthChange={() => { statsRefreshKey += 1; }}
  />

  <!-- Application (desktop only) -->
  {#if !isMobile}
    <ApplicationSection bind:autostartEnabled />
  {/if}

  <!-- Save button (Last.fm keys + providers) -->
  <div class="flex justify-end">
    <button
      class="flex items-center gap-2 rounded-lg bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground shadow transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
      onclick={save}
      disabled={saving}
    >
      {#if saving}
        <Loader2 class="size-4 animate-spin" />
        Saving…
      {:else}
        <Save class="size-4" />
        Save Changes
      {/if}
    </button>
  </div>

  <!-- Danger Zone -->
  <DangerZoneSection onCleared={handleCleared} />
</div>
