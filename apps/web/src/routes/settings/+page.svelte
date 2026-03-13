<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Settings,
    Server,
    Music2,
    Heart,
    ListMusic,
    Star,
    Eye,
    EyeOff,
    Save,
    RefreshCw,
    CheckCircle2,
    XCircle,
    AlertCircle,
    Loader2,
    Plus,
    Pencil,
    Trash2,
    CheckCircle,
    Link,
    Unlink,
    ExternalLink,
    User
  } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { appSettings, libraryRefresh } from '$lib/stores/settings';
  import * as Select from '$lib/components/ui/select';
  import * as Dialog from '$lib/components/ui/dialog';
  import {
    activateProfile as activateProfileApi,
    createProfile,
    deleteProfile as deleteProfileApi,
    fetchAppSettings,
    fetchLastFmStatus,
    fetchLibraryStats,
    fetchProfiles,
    fetchServiceHealth,
    lfmBeginAuth,
    lfmCompleteAuth,
    lfmDisconnect,
    type LibraryStatsPayload,
    type ProfileDraftPayload,
    type ProfilePayload,
    type ServiceStatus,
    updateAppSettings,
    updateProfile
  } from '$lib/api';

  type Profile = ProfilePayload;

  type ProfileDraft = {
    id: number | null;
    name: string;
    url: string;
    username: string;
    password: string;
    usePasswordAuth: boolean;
  };

  // ── Form state ────────────────────────────────────────────────────────────
  let lastFmApiKey = $state('');
  let lastFmSharedSecret = $state('');
  let lastFmSharedSecretConfigured = $state(false);
  let showSharedSecret = $state(false);
  let recommendationProvider = $state('lastfm');
  let metadataProvider = $state('both');
  let showApiKey = $state(false);

  // ── Last.fm account state ─────────────────────────────────────────────────
  let lfmConnected = $state(false);
  let lfmUsername = $state('');
  let lfmAuthState = $state<'idle' | 'pending' | 'completing'>('idle');
  let lfmPendingToken = $state('');
  let lfmConnecting = $state(false);
  let lfmDisconnecting = $state(false);

  // Sync connected state into the global store so PlayerBar can scrobble
  $effect(() => {
    appSettings.update(s => ({ ...s, lastFmConnected: lfmConnected, lastFmUsername: lfmUsername }));
  });

  // ── Profiles state ────────────────────────────────────────────────────────
  let profiles = $state<Profile[]>([]);
  let profileDraft = $state<ProfileDraft | null>(null);
  let dialogOpen = $state(false);
  let showDraftPassword = $state(false);
  let savingProfile = $state(false);

  // Clear draft whenever dialog closes (X button, overlay click, or cancel)
  $effect(() => { if (!dialogOpen) profileDraft = null; });

  // ── Save state ────────────────────────────────────────────────────────────
  let saving = $state(false);

  // ── Connection status ─────────────────────────────────────────────────────
  let subsonicStatus = $state<ServiceStatus>('checking');
  let lastfmStatus = $state<ServiceStatus>('checking');
  let recheckingHealth = $state(false);

  // ── Library stats ─────────────────────────────────────────────────────────
  let stats = $state<LibraryStatsPayload | null>(null);
  let statsLoading = $state(true);

  onMount(async () => {
    await Promise.all([loadInitialSettings(), fetchHealth(), fetchStats()]);
  });

  async function loadInitialSettings() {
    try {
      const [settings, loadedProfiles, lfmStatus] = await Promise.all([
        fetchAppSettings(),
        fetchProfiles(),
        fetchLastFmStatus().catch(() => ({ connected: false, username: '' }))
      ]);

      lastFmApiKey = settings.lastFmApiKey;
      lastFmSharedSecretConfigured = settings.lastFmSharedSecretConfigured;
      recommendationProvider = settings.recommendationProvider;
      metadataProvider = settings.metadataProvider;
      profiles = loadedProfiles;
      lfmConnected = lfmStatus.connected;
      lfmUsername = lfmStatus.username;

      appSettings.set({
        lastFmApiKey,
        recommendationProvider,
        metadataProvider,
        lastFmConnected: lfmConnected,
        lastFmUsername: lfmUsername
      });
    } catch {
      toast.error('Failed to load settings');
    }
  }

  async function fetchHealth() {
    try {
      const payload = await fetchServiceHealth();
      subsonicStatus = payload?.subsonic ?? 'offline';
      lastfmStatus = payload?.lastfm ?? 'offline';
    } catch {
      subsonicStatus = 'offline';
      lastfmStatus = 'offline';
    }
  }

  async function fetchStats() {
    statsLoading = true;
    try {
      stats = await fetchLibraryStats();
    } catch {
      stats = null;
    } finally {
      statsLoading = false;
    }
  }

  async function recheckConnections() {
    recheckingHealth = true;
    subsonicStatus = 'checking';
    lastfmStatus = 'checking';
    await fetchHealth();
    recheckingHealth = false;
  }

  async function save() {
    saving = true;
    try {
      await updateAppSettings({
        lastFmApiKey,
        recommendationProvider,
        metadataProvider,
        lastFmSharedSecret
      });

      appSettings.set({ lastFmApiKey, recommendationProvider, metadataProvider, lastFmConnected: lfmConnected, lastFmUsername: lfmUsername });
      libraryRefresh.update((n) => n + 1);
      if (lastFmSharedSecret.trim()) {
        lastFmSharedSecretConfigured = true;
        lastFmSharedSecret = '';
      }
      toast.success('Settings saved');
      await Promise.all([fetchHealth(), fetchStats()]);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to save settings');
    } finally {
      saving = false;
    }
  }

  // ── Last.fm account helpers ───────────────────────────────────────

  async function connectLastFm() {
    lfmConnecting = true;
    try {
      const { token, authUrl } = await lfmBeginAuth();
      lfmPendingToken = token;
      lfmAuthState = 'pending';
      // Open the Last.fm auth page in the system browser
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(authUrl).catch(() => window.open(authUrl, '_blank'));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to start Last.fm auth');
    } finally {
      lfmConnecting = false;
    }
  }

  async function completeLastFmAuth() {
    if (!lfmPendingToken) return;
    lfmAuthState = 'completing';
    try {
      const { username } = await lfmCompleteAuth(lfmPendingToken);
      lfmConnected = true;
      lfmUsername = username;
      lfmAuthState = 'idle';
      lfmPendingToken = '';
      toast.success(`Connected as ${username}`);
      await fetchHealth();
    } catch (err) {
      lfmAuthState = 'pending'; // let them retry
      toast.error(err instanceof Error ? err.message : 'Authorization failed — make sure you approved the app on Last.fm');
    }
  }

  function cancelLastFmAuth() {
    lfmAuthState = 'idle';
    lfmPendingToken = '';
  }

  async function disconnectLastFm() {
    lfmDisconnecting = true;
    try {
      await lfmDisconnect();
      lfmConnected = false;
      lfmUsername = '';
      toast.success('Disconnected from Last.fm');
    } catch {
      toast.error('Failed to disconnect');
    } finally {
      lfmDisconnecting = false;
    }
  }

  // ── Label maps for Select components ─────────────────────────────────────
  const META_LABELS: Record<string, string> = {
    both: 'Both (Last.fm + TheAudioDB)',
    lastfm: 'Last.fm only',
    audiodb: 'TheAudioDB only'
  };
  const REC_LABELS: Record<string, string> = { lastfm: 'Last.fm' };

  // ── Profile helpers ───────────────────────────────────────────────────────
  function openAdd() {
    profileDraft = { id: null, name: '', url: '', username: '', password: '', usePasswordAuth: false };
    showDraftPassword = false;
    dialogOpen = true;
  }

  function openEdit(p: Profile) {
    profileDraft = { id: p.id, name: p.name, url: p.url, username: p.username, password: '', usePasswordAuth: p.usePasswordAuth };
    showDraftPassword = false;
    dialogOpen = true;
  }

  function cancelDraft() {
    dialogOpen = false;
  }

  async function saveProfile() {
    if (!profileDraft) return;
    const isNew = profileDraft.id === null;
    if (isNew && !profileDraft.password) {
      toast.error('Password is required for a new server');
      return;
    }
    savingProfile = true;
    try {
      const body: ProfileDraftPayload = {
        name: profileDraft.name,
        url: profileDraft.url,
        username: profileDraft.username,
        usePasswordAuth: profileDraft.usePasswordAuth
      };
      if (profileDraft.password) {
        body.password = profileDraft.password;
      }

      const profile = isNew
        ? await createProfile(body)
        : await updateProfile(profileDraft.id as number, body);

      if (isNew) {
        profiles = [...profiles, profile];
      } else {
        profiles = profiles.map(p => p.id === profile.id ? profile : p);
      }
      dialogOpen = false;
      toast.success(isNew ? 'Server added' : 'Server updated');
      await fetchHealth();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to save server');
    } finally {
      savingProfile = false;
    }
  }

  async function activateProfile(id: number) {
    try {
      await activateProfileApi(id);
      profiles = profiles.map(p => ({ ...p, isActive: p.id === id }));
      libraryRefresh.update((n) => n + 1);
      toast.success('Server activated');
      await fetchHealth();
    } catch {
      toast.error('Failed to activate server');
    }
  }

  async function deleteProfile(id: number) {
    const prof = profiles.find(p => p.id === id);
    if (!prof) return;
    if (prof.isActive && profiles.length === 1) {
      toast.error('Cannot delete the only server');
      return;
    }
    try {
      await deleteProfileApi(id);
      const wasActive = prof.isActive;
      profiles = profiles.filter(p => p.id !== id);
      if (wasActive && profiles.length > 0) {
        profiles = profiles.map((p, i) => i === 0 ? { ...p, isActive: true } : p);
      }
      toast.success('Server removed');
      if (wasActive) await fetchHealth();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to delete server');
    }
  }

  function statusIcon(status: ServiceStatus) {
    return { checking: Loader2, online: CheckCircle2, offline: XCircle, missing: AlertCircle }[status];
  }

  function statusClass(status: ServiceStatus): string {
    if (status === 'online') return 'text-emerald-400';
    if (status === 'missing') return 'text-amber-400';
    if (status === 'offline') return 'text-rose-400';
    return 'text-muted-foreground';
  }

  function fmt(n: number | null | undefined): string {
    if (n === null || n === undefined) return '—';
    return n.toLocaleString();
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

  <!-- ── Connection status ───────────────────────────────────────────────── -->
  <section>
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Service Status</h2>
      <button
        class="flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs text-muted-foreground transition hover:bg-secondary hover:text-foreground disabled:opacity-50"
        onclick={recheckConnections}
        disabled={recheckingHealth}
      >
        <RefreshCw class="size-3.5 {recheckingHealth ? 'animate-spin' : ''}" />
        Recheck
      </button>
    </div>
    <div class="grid grid-cols-2 gap-3">
      {#each [{ label: 'Subsonic', status: subsonicStatus }, { label: 'Last.fm', status: lastfmStatus }] as svc (svc.label)}
        {@const Icon = statusIcon(svc.status)}
        <div class="flex items-center gap-3 rounded-xl border border-border/70 bg-card px-4 py-3.5">
          <Server class="size-5 shrink-0 text-muted-foreground" />
          <div class="min-w-0 flex-1">
            <p class="text-sm font-semibold">{svc.label}</p>
            <p class="text-xs text-muted-foreground">
              {svc.status === 'online' ? 'Connected' : svc.status === 'checking' ? 'Checking…' : svc.status === 'missing' ? 'Not configured' : 'Unreachable'}
            </p>
          </div>
          <Icon class="size-5 shrink-0 {statusClass(svc.status)} {svc.status === 'checking' ? 'animate-spin' : ''}" />
        </div>
      {/each}
    </div>
  </section>

  <!-- ── Servers ────────────────────────────────────────────────────────── -->
  <section class="rounded-xl border border-border/70 bg-card">
    <div class="flex items-center justify-between border-b border-border/60 px-5 py-4">
      <div>
        <div class="flex items-center gap-2">
          <Server class="size-4 text-muted-foreground" />
          <h2 class="font-semibold">Servers</h2>
        </div>
        <p class="mt-0.5 text-xs text-muted-foreground">Manage your Subsonic / Navidrome connections.</p>
      </div>
      <button
        class="flex items-center gap-1.5 rounded-lg border border-border bg-secondary/60 px-3 py-1.5 text-xs font-medium transition hover:bg-secondary"
        onclick={openAdd}
      >
        <Plus class="size-3.5" />
        Add Server
      </button>
    </div>

    <!-- Profile list -->
    {#if profiles.length === 0}
      <div class="px-5 py-10 text-center">
        <Server class="mx-auto mb-3 size-8 text-muted-foreground/40" />
        <p class="text-sm font-medium text-muted-foreground">No servers configured</p>
        <p class="mt-1 text-xs text-muted-foreground/70">Add your first Subsonic or Navidrome server above.</p>
      </div>
    {:else}
      <ul class="divide-y divide-border/50">
        {#each profiles as profile (profile.id)}
          <li class="flex items-center gap-3 px-5 py-3.5">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="truncate text-sm font-medium">{profile.name}</span>
                {#if profile.isActive}
                  <span class="shrink-0 rounded-full bg-emerald-500/15 px-2 py-0.5 text-xs font-medium text-emerald-400">Active</span>
                {/if}
              </div>
              <p class="truncate text-xs text-muted-foreground">{profile.url}</p>
              <p class="text-xs text-muted-foreground/70">{profile.username}</p>
            </div>
            <div class="flex shrink-0 items-center gap-1">
              {#if !profile.isActive}
                <button
                  class="flex items-center gap-1 rounded-md px-2.5 py-1.5 text-xs text-muted-foreground transition hover:bg-secondary hover:text-foreground"
                  onclick={() => activateProfile(profile.id)}
                  title="Set as active server"
                >
                  <CheckCircle class="size-3.5" />
                  Activate
                </button>
              {/if}
              <button
                class="rounded-md p-1.5 text-muted-foreground transition hover:bg-secondary hover:text-foreground"
                onclick={() => openEdit(profile)}
                title="Edit server"
              >
                <Pencil class="size-3.5" />
              </button>
              <button
                class="rounded-md p-1.5 text-muted-foreground transition hover:bg-secondary hover:text-rose-400 disabled:pointer-events-none disabled:opacity-40"
                onclick={() => deleteProfile(profile.id)}
                disabled={profile.isActive && profiles.length === 1}
                title="Delete server"
              >
                <Trash2 class="size-3.5" />
              </button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}

  </section>

  <!-- ── Server dialog ─────────────────────────────────────────────────── -->
  <Dialog.Root bind:open={dialogOpen}>
    <Dialog.Content class="sm:max-w-xl">
      <Dialog.Header>
        <Dialog.Title>{profileDraft?.id === null ? 'Add Server' : 'Edit Server'}</Dialog.Title>
        <Dialog.Description>Configure your Subsonic / Navidrome connection details.</Dialog.Description>
      </Dialog.Header>
      {#if profileDraft !== null}
        <div class="space-y-4 py-2">
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-1.5">
              <label class="text-sm font-medium" for="dialog-name">Name</label>
              <input
                id="dialog-name"
                type="text"
                bind:value={profileDraft.name}
                placeholder="Home Server"
                class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
            <div class="space-y-1.5">
              <label class="text-sm font-medium" for="dialog-url">Server URL</label>
              <input
                id="dialog-url"
                type="url"
                bind:value={profileDraft.url}
                placeholder="http://localhost:4533"
                class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-1.5">
              <label class="text-sm font-medium" for="dialog-user">Username</label>
              <input
                id="dialog-user"
                type="text"
                bind:value={profileDraft.username}
                placeholder="admin"
                autocomplete="username"
                class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
            <div class="space-y-1.5">
              <label class="text-sm font-medium" for="dialog-pass">
                {profileDraft.id !== null ? 'Password (blank = keep current)' : 'Password'}
              </label>
              <div class="relative">
                <input
                  id="dialog-pass"
                  type={showDraftPassword ? 'text' : 'password'}
                  bind:value={profileDraft.password}
                  placeholder={profileDraft.id !== null ? 'Leave blank to keep current' : 'Password'}
                  autocomplete="current-password"
                  class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 pr-10 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                />
                <button
                  type="button"
                  class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  onclick={() => { showDraftPassword = !showDraftPassword; }}
                  aria-label={showDraftPassword ? 'Hide password' : 'Show password'}
                >
                  {#if showDraftPassword}
                    <EyeOff class="size-4" />
                  {:else}
                    <Eye class="size-4" />
                  {/if}
                </button>
              </div>
            </div>
          </div>
          <label class="flex cursor-pointer items-center gap-3" for="dialog-auth-toggle">
            <input
              id="dialog-auth-toggle"
              type="checkbox"
              class="sr-only"
              bind:checked={profileDraft.usePasswordAuth}
            />
            <div
              aria-hidden="true"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {profileDraft.usePasswordAuth ? 'bg-primary' : 'bg-input'}"
            >
              <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {profileDraft.usePasswordAuth ? 'translate-x-4' : ''}"></span>
            </div>
            <span class="text-sm">Use plaintext password auth (legacy servers)</span>
          </label>
        </div>
      {/if}
      <Dialog.Footer>
        <button
          class="rounded-lg border border-border px-4 py-2 text-sm transition hover:bg-secondary disabled:opacity-50"
          onclick={cancelDraft}
          disabled={savingProfile}
        >
          Cancel
        </button>
        <button
          class="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90 disabled:opacity-60"
          onclick={saveProfile}
          disabled={savingProfile}
        >
          {#if savingProfile}
            <Loader2 class="size-4 animate-spin" />
            Saving…
          {:else}
            <Save class="size-4" />
            {profileDraft?.id === null ? 'Add Server' : 'Save Changes'}
          {/if}
        </button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>

  <!-- ── Last.fm ─────────────────────────────────────────────────────────── -->
  <section class="rounded-xl border border-border/70 bg-card">
    <div class="border-b border-border/60 px-5 py-4">
      <div class="flex items-center gap-2">
        <Music2 class="size-4 text-muted-foreground" />
        <h2 class="font-semibold">Last.fm</h2>
      </div>
      <p class="mt-0.5 text-xs text-muted-foreground">
        Required for recommendations, radio, and artist info.
        Get a free API key at
        <a href="https://www.last.fm/api/account/create" target="_blank" rel="noopener noreferrer" class="text-primary underline-offset-2 hover:underline">last.fm/api</a>.
      </p>
    </div>
    <div class="space-y-5 px-5 py-5">

      <!-- API Key -->
      <div class="space-y-1.5">
        <label class="text-sm font-medium" for="lfm-key">API Key</label>
        <div class="relative">
          <input
            id="lfm-key"
            type={showApiKey ? 'text' : 'password'}
            bind:value={lastFmApiKey}
            placeholder="Paste your Last.fm API key"
            autocomplete="off"
            class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 pr-10 font-mono text-sm placeholder:font-sans placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
          <button
            type="button"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onclick={() => { showApiKey = !showApiKey; }}
            aria-label={showApiKey ? 'Hide key' : 'Show key'}
          >
            {#if showApiKey}<EyeOff class="size-4" />{:else}<Eye class="size-4" />{/if}
          </button>
        </div>
      </div>

      <!-- Shared Secret -->
      <div class="space-y-1.5">
        <label class="text-sm font-medium" for="lfm-secret">
          Shared Secret
          <span class="ml-1.5 text-xs font-normal text-muted-foreground">Required for scrobbling &amp; account linking</span>
        </label>
        <div class="relative">
          <input
            id="lfm-secret"
            type={showSharedSecret ? 'text' : 'password'}
            bind:value={lastFmSharedSecret}
            placeholder={lastFmSharedSecretConfigured ? 'Already configured — enter to replace' : 'Paste your Last.fm shared secret'}
            autocomplete="off"
            class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 pr-10 font-mono text-sm placeholder:font-sans placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
          <button
            type="button"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onclick={() => { showSharedSecret = !showSharedSecret; }}
            aria-label={showSharedSecret ? 'Hide secret' : 'Show secret'}
          >
            {#if showSharedSecret}<EyeOff class="size-4" />{:else}<Eye class="size-4" />{/if}
          </button>
        </div>
      </div>

      <!-- Account Connection -->
      <div class="space-y-3 rounded-lg border border-border/60 bg-secondary/20 px-4 py-4">
        <p class="text-sm font-medium">Account</p>

        {#if lfmConnected}
          <!-- Connected state -->
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2.5 text-sm">
              <div class="flex size-7 items-center justify-center rounded-full bg-emerald-500/15 text-emerald-500">
                <User class="size-3.5" />
              </div>
              <div>
                <p class="font-medium leading-none">{lfmUsername}</p>
                <p class="mt-0.5 text-xs text-muted-foreground">Connected · scrobbling enabled</p>
              </div>
            </div>
            <button
              class="flex items-center gap-1.5 rounded-md border border-border/70 px-3 py-1.5 text-xs font-medium text-muted-foreground transition hover:border-destructive/60 hover:bg-destructive/10 hover:text-destructive disabled:opacity-50"
              onclick={disconnectLastFm}
              disabled={lfmDisconnecting}
            >
              {#if lfmDisconnecting}
                <Loader2 class="size-3.5 animate-spin" />
              {:else}
                <Unlink class="size-3.5" />
              {/if}
              Disconnect
            </button>
          </div>

        {:else if lfmAuthState === 'pending' || lfmAuthState === 'completing'}
          <!-- Pending authorization in browser -->
          <div class="space-y-3">
            <div class="flex items-start gap-2.5 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2.5 text-sm text-amber-600 dark:text-amber-400">
              <ExternalLink class="mt-0.5 size-3.5 shrink-0" />
              <span>Authorize the app in your browser, then click the button below.</span>
            </div>
            <div class="flex gap-2">
              <button
                class="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground transition hover:bg-primary/90 disabled:opacity-50"
                onclick={completeLastFmAuth}
                disabled={lfmAuthState === 'completing'}
              >
                {#if lfmAuthState === 'completing'}
                  <Loader2 class="size-4 animate-spin" />
                  Verifying…
                {:else}
                  <Link class="size-4" />
                  I've Authorized — Connect
                {/if}
              </button>
              <button
                class="rounded-md border border-border/70 px-3 py-2 text-sm text-muted-foreground transition hover:bg-secondary"
                onclick={cancelLastFmAuth}
                disabled={lfmAuthState === 'completing'}
              >
                Cancel
              </button>
            </div>
          </div>

        {:else}
          <!-- Idle: not connected -->
          <div class="flex items-center justify-between gap-3">
            <p class="text-xs text-muted-foreground">
              Link your Last.fm account to enable scrobbling and personalised recommendations based on your listening history.
            </p>
            <button
              class="flex shrink-0 items-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground transition hover:bg-primary/90 disabled:opacity-50"
              onclick={connectLastFm}
              disabled={lfmConnecting || !lastFmSharedSecretConfigured && !lastFmSharedSecret.trim()}
            >
              {#if lfmConnecting}
                <Loader2 class="size-4 animate-spin" />
                Opening…
              {:else}
                <Link class="size-4" />
                Connect Account
              {/if}
            </button>
          </div>
          {#if !lastFmSharedSecretConfigured && !lastFmSharedSecret.trim()}
            <p class="text-xs text-amber-500">Enter and save your shared secret above before connecting.</p>
          {/if}
        {/if}
      </div>

    </div>
  </section>

  <!-- ── Providers ──────────────────────────────────────────────────────── -->
  <section class="rounded-xl border border-border/70 bg-card">
    <div class="border-b border-border/60 px-5 py-4">
      <h2 class="font-semibold">Providers</h2>
      <p class="mt-0.5 text-xs text-muted-foreground">Choose which data sources power metadata and recommendations.</p>
    </div>
    <div class="grid grid-cols-2 gap-4 px-5 py-5">
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Metadata Provider</p>
        <Select.Root bind:value={metadataProvider}>
          <Select.Trigger class="w-full">
            {META_LABELS[metadataProvider] ?? metadataProvider}
          </Select.Trigger>
          <Select.Content>
            <Select.Item value="both" label="Both (Last.fm + TheAudioDB)" />
            <Select.Item value="lastfm" label="Last.fm only" />
            <Select.Item value="audiodb" label="TheAudioDB only" />
          </Select.Content>
        </Select.Root>
      </div>
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Recommendation Provider</p>
        <Select.Root bind:value={recommendationProvider}>
          <Select.Trigger class="w-full">
            {REC_LABELS[recommendationProvider] ?? recommendationProvider}
          </Select.Trigger>
          <Select.Content>
            <Select.Item value="lastfm" label="Last.fm" />
          </Select.Content>
        </Select.Root>
      </div>
    </div>
  </section>

  <!-- ── Save button ────────────────────────────────────────────────────── -->
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

  <!-- ── Library stats ──────────────────────────────────────────────────── -->
  <section>
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Library Stats</h2>
      <button
        class="flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs text-muted-foreground transition hover:bg-secondary hover:text-foreground disabled:opacity-50"
        onclick={fetchStats}
        disabled={statsLoading}
      >
        <RefreshCw class="size-3.5 {statsLoading ? 'animate-spin' : ''}" />
        Refresh
      </button>
    </div>

    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
      {#each [
        { label: 'Saved Artists', value: stats?.likedArtists, icon: Heart, color: 'text-rose-400' },
        { label: 'Starred Songs', value: stats?.starredSongs, icon: Star, color: 'text-amber-400' },
        { label: 'Playlists', value: stats?.playlistCount, icon: ListMusic, color: 'text-sky-400' },
        { label: 'Playlist Tracks', value: stats?.totalPlaylistSongs, icon: Music2, color: 'text-violet-400' }
      ] as card (card.label)}
        {@const Icon = card.icon}
        <div class="flex flex-col gap-2 rounded-xl border border-border/70 bg-card px-4 py-4">
          <Icon class="size-5 {card.color}" />
          <div>
            {#if statsLoading}
              <div class="h-7 w-12 animate-pulse rounded bg-secondary"></div>
            {:else}
              <p class="text-2xl font-bold tabular-nums">{fmt(card.value)}</p>
            {/if}
            <p class="text-xs text-muted-foreground">{card.label}</p>
          </div>
        </div>
      {/each}
    </div>

    {#if stats && stats.starredSongs === null}
      <p class="mt-2 text-xs text-muted-foreground">Some stats are unavailable — Subsonic may not be connected.</p>
    {/if}
  </section>
</div>
