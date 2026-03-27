<script lang="ts">
  import { Server, Plus, Pencil, Trash2, CheckCircle, Eye, EyeOff, Save, Loader2, Link2, Wifi } from '@lucide/svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { toast } from 'svelte-sonner';
  import {
    activateProfile as activateProfileApi,
    createProfile,
    deleteProfile as deleteProfileApi,
    plexBeginAuth,
    plexPollAuth,
    testProfileConnection,
    updateProfile,
    type PlexServerResourcePayload,
    type ProfileDraftPayload,
    type ProfilePayload,
  } from '$lib/servers';
  import { openUrl } from '$lib/tauri';
  import { goto } from '$app/navigation';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';

  type Profile = ProfilePayload;

  type ProfileDraft = {
    id: number | null;
    name: string;
    url: string;
    username: string;
    password: string;
    serverType: string;
  };

  const SERVER_TYPE_LABELS: Record<string, string> = {
    subsonic: 'Subsonic / OpenSubsonic',
    subsonic_legacy: 'Subsonic Legacy',
    jellyfin: 'Jellyfin',
    emby: 'Emby',
    plex: 'Plex',
    local: 'Local Folder',
    octo_fiesta: 'Octo Fiesta',
  };

  let {
    profiles = $bindable<Profile[]>([]),
    onHealthChange,
  }: {
    profiles: Profile[];
    onHealthChange: () => void;
  } = $props();

  let profileDraft = $state<ProfileDraft | null>(null);
  let dialogOpen = $state(false);
  let showDraftPassword = $state(false);
  let savingProfile = $state(false);
  let testingConnection = $state(false);
  let plexAuthPending = $state(false);
  let plexAuthCode = $state('');
  let plexAuthUsername = $state('');
  let plexSelectedServerUri = $state('');
  let plexServers = $state<PlexServerResourcePayload[]>([]);
  let plexAuthAttempt = 0;

  function resetPlexAuthState() {
    plexAuthPending = false;
    plexAuthCode = '';
    plexAuthUsername = '';
    plexSelectedServerUri = '';
    plexServers = [];
    plexAuthAttempt += 1;
  }

  function applySelectedPlexServer(uri: string) {
    if (!profileDraft) return;
    const server = plexServers.find((item) => item.uri === uri);
    if (!server) return;
    plexSelectedServerUri = server.uri;
    profileDraft.url = server.uri;
    profileDraft.password = server.accessToken;
    if (!profileDraft.name.trim() || profileDraft.name === 'Plex') {
      profileDraft.name = server.name;
    }
  }

  async function beginPlexConnect() {
    if (!profileDraft || profileDraft.serverType !== 'plex') return;
    const attempt = plexAuthAttempt + 1;
    plexAuthAttempt = attempt;
    plexAuthPending = true;
    plexAuthCode = '';
    plexAuthUsername = '';
    plexSelectedServerUri = '';
    plexServers = [];

    try {
      const start = await plexBeginAuth();
      if (attempt !== plexAuthAttempt) return;
      plexAuthCode = start.code;
      await openUrl(start.authUrl);

      for (let tries = 0; tries < 90; tries += 1) {
        await new Promise((resolve) => window.setTimeout(resolve, 2000));
        if (attempt !== plexAuthAttempt || !dialogOpen || !profileDraft || profileDraft.serverType !== 'plex') return;

        const result = await plexPollAuth(start.pinId);
        if (attempt !== plexAuthAttempt || !profileDraft || profileDraft.serverType !== 'plex') return;

        if (result.status === 'complete') {
          plexAuthPending = false;
          plexAuthUsername = result.username ?? '';
          plexServers = result.servers;
          if (result.username) {
            profileDraft.username = result.username;
          }
          if (plexServers.length > 0) {
            applySelectedPlexServer(plexServers[0].uri);
            toast.success('Plex account connected');
          } else {
            toast.error('Connected to Plex, but no servers were available for this account');
          }
          return;
        }

        if (result.status === 'expired') {
          plexAuthPending = false;
          toast.error('Plex sign-in expired. Start the connection again.');
          return;
        }
      }

      plexAuthPending = false;
      toast.error('Timed out waiting for Plex sign-in');
    } catch (err) {
      plexAuthPending = false;
      toast.error(err instanceof Error ? err.message : 'Failed to connect Plex');
    }
  }

  $effect(() => {
    if (!dialogOpen) {
      profileDraft = null;
      testingConnection = false;
      resetPlexAuthState();
    }
  });

  $effect(() => {
    if (!profileDraft || profileDraft.serverType !== 'plex') {
      resetPlexAuthState();
    }
  });

  function openAdd() {
    profileDraft = { id: null, name: '', url: '', username: '', password: '', serverType: 'subsonic' };
    showDraftPassword = false;
    resetPlexAuthState();
    dialogOpen = true;
  }

  function openEdit(p: Profile) {
    profileDraft = { id: p.id, name: p.name, url: p.url, username: p.username, password: '', serverType: p.serverType };
    showDraftPassword = false;
    resetPlexAuthState();
    dialogOpen = true;
  }

  function cancelDraft() {
    dialogOpen = false;
  }

  async function testConnection() {
    if (!profileDraft) return;
    if (profileDraft.serverType !== 'local' && !profileDraft.password && profileDraft.id !== null) {
      toast.error('Re-enter your password to test an existing server');
      return;
    }
    testingConnection = true;
    try {
      await testProfileConnection({
        url: profileDraft.url,
        username: profileDraft.username,
        password: profileDraft.password,
        serverType: profileDraft.serverType,
      });
      toast.success('Connection successful');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err) || 'Connection failed');
    } finally {
      testingConnection = false;
    }
  }

  async function saveProfile() {
    if (!profileDraft) return;
    const isNew = profileDraft.id === null;
    if (profileDraft.serverType !== 'local' && isNew && !profileDraft.password) {
      toast.error('Password is required for a new server');
      return;
    }
    savingProfile = true;
    try {
      const body: ProfileDraftPayload = {
        name: profileDraft.name,
        url: profileDraft.url,
        username: profileDraft.username,
        serverType: profileDraft.serverType,
      };
      if (profileDraft.password) body.password = profileDraft.password;

      const profile = isNew
        ? await createProfile(body)
        : await updateProfile(profileDraft.id as number, body);

      if (isNew) {
        profiles = [...profiles, profile];
      } else {
        profiles = profiles.map((p) => (p.id === profile.id ? profile : p));
      }
      requestLibraryRefresh();
      dialogOpen = false;
      toast.success(isNew ? 'Server added' : 'Server updated');
      onHealthChange();
      if (isNew) goto('/');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to save server');
    } finally {
      savingProfile = false;
    }
  }

  async function activateProfile(id: number) {
    try {
      await activateProfileApi(id);
      profiles = profiles.map((p) => ({ ...p, isActive: p.id === id }));
      requestLibraryRefresh();
      toast.success('Server activated');
      onHealthChange();
    } catch {
      toast.error('Failed to activate server');
    }
  }

  async function deleteProfile(id: number) {
    const prof = profiles.find((p) => p.id === id);
    if (!prof) return;
    if (prof.isActive && profiles.length === 1) {
      toast.error('Cannot delete the only server');
      return;
    }
    try {
      await deleteProfileApi(id);
      const wasActive = prof.isActive;
      profiles = profiles.filter((p) => p.id !== id);
      if (wasActive && profiles.length > 0) {
        profiles = profiles.map((p, i) => (i === 0 ? { ...p, isActive: true } : p));
      }
      requestLibraryRefresh();
      toast.success('Server removed');
      if (wasActive) onHealthChange();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to delete server');
    }
  }
</script>

<section class="rounded-xl border border-border/70 bg-card">
  <div class="flex items-center justify-between border-b border-border/60 px-5 py-4">
    <div>
      <div class="flex items-center gap-2">
        <Server class="size-4 text-muted-foreground" />
        <h2 class="font-semibold">Servers</h2>
      </div>
      <p class="mt-0.5 text-xs text-muted-foreground">Manage your Subsonic-compatible, Jellyfin, Emby, Plex, and local library connections.</p>
    </div>
    <button
      class="flex items-center gap-1.5 rounded-lg border border-border bg-secondary/60 px-3 py-1.5 text-xs font-medium transition hover:bg-secondary"
      onclick={openAdd}
    >
      <Plus class="size-3.5" />
      Add Server
    </button>
  </div>

  {#if profiles.length === 0}
    <div class="px-5 py-10 text-center">
      <Server class="mx-auto mb-3 size-8 text-muted-foreground/40" />
      <p class="text-sm font-medium text-muted-foreground">No servers configured</p>
      <p class="mt-1 text-xs text-muted-foreground/70">Add your first Subsonic-compatible, Jellyfin, Emby, Plex, or local library above.</p>
    </div>
  {:else}
    <ul class="divide-y divide-border/50">
      {#each profiles as profile (profile.id)}
        <li class="flex items-center gap-3 px-5 py-3.5">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate text-sm font-medium">{profile.name}</span>
              <span class="shrink-0 rounded-full bg-secondary px-2 py-0.5 text-xs text-muted-foreground">
                {SERVER_TYPE_LABELS[profile.serverType] ?? profile.serverType}
              </span>
              {#if profile.isActive}
                <span class="shrink-0 rounded-full bg-emerald-500/15 px-2 py-0.5 text-xs font-medium text-emerald-400">Active</span>
              {/if}
            </div>
            <p class="truncate text-xs text-muted-foreground">{profile.url}</p>
            {#if profile.serverType !== 'local'}
              <p class="text-xs text-muted-foreground/70">{profile.username}</p>
            {/if}
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

<Dialog.Root bind:open={dialogOpen}>
  <Dialog.Content class="sm:max-w-xl">
    <Dialog.Header>
      <Dialog.Title>{profileDraft?.id === null ? 'Add Server' : 'Edit Server'}</Dialog.Title>
      <Dialog.Description>Configure your server connection. Subsonic-compatible servers like Navidrome and Octo-Fiesta use the Subsonic options. Jellyfin, Emby, and Plex use an API or auth token as the password. Local Folder uses a filesystem path.</Dialog.Description>
    </Dialog.Header>
    {#if profileDraft !== null}
      <form id="server-profile-form" onsubmit={(e) => { e.preventDefault(); saveProfile(); }}>
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
            <label class="text-sm font-medium" for="dialog-server-type">Server Type</label>
            <select
              id="dialog-server-type"
              bind:value={profileDraft.serverType}
              class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="subsonic">Subsonic / OpenSubsonic</option>
              <option value="subsonic_legacy">Subsonic Legacy</option>
              <option value="jellyfin">Jellyfin</option>
              <option value="emby">Emby</option>
              <option value="plex">Plex</option>
              <option value="local">Local Folder</option>
            </select>
          </div>
        </div>
        <div class="space-y-1.5">
          <label class="text-sm font-medium" for="dialog-url">{profileDraft.serverType === 'local' ? 'Library Folder' : 'Server URL'}</label>
          <input
            id="dialog-url"
            type={profileDraft.serverType === 'local' ? 'text' : 'url'}
            bind:value={profileDraft.url}
            placeholder={profileDraft.serverType === 'local'
              ? 'D:\\Music or /home/user/Music'
              : (profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby')
                ? 'http://localhost:8096'
                : profileDraft.serverType === 'plex'
                  ? 'http://localhost:32400'
                : 'http://localhost:4533'}
            class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        {#if profileDraft.serverType === 'plex'}
          <div class="rounded-xl border border-border/70 bg-secondary/30 p-3">
            <div class="flex items-start justify-between gap-3">
              <div class="space-y-1">
                <p class="text-sm font-medium">Connect Plex Account</p>
                <p class="text-xs text-muted-foreground">
                  Sign in with Plex, then choose one of the servers linked to that account. You can still enter a server URL and token manually if you prefer.
                </p>
              </div>
              <button
                type="button"
                class="inline-flex shrink-0 items-center gap-2 rounded-lg border border-border bg-background/40 px-3 py-2 text-xs font-medium transition hover:bg-background/60 disabled:opacity-60"
                onclick={beginPlexConnect}
                disabled={plexAuthPending || savingProfile}
              >
                {#if plexAuthPending}
                  <Loader2 class="size-3.5 animate-spin" />
                  Waiting…
                {:else}
                  <Link2 class="size-3.5" />
                  Connect Plex
                {/if}
              </button>
            </div>
            {#if plexAuthCode}
              <div class="mt-3 rounded-lg border border-border/60 bg-background/40 px-3 py-2">
                <p class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Link code</p>
                <p class="mt-1 text-lg font-semibold tracking-[0.28em]">{plexAuthCode}</p>
                <p class="mt-1 text-xs text-muted-foreground">A browser window should open to Plex sign-in automatically.</p>
              </div>
            {/if}
            {#if plexAuthUsername || plexServers.length > 0}
              <div class="mt-3 space-y-3">
                {#if plexAuthUsername}
                  <p class="text-xs text-muted-foreground">Connected as <span class="font-medium text-foreground">{plexAuthUsername}</span></p>
                {/if}
                {#if plexServers.length > 0}
                  <div class="space-y-1.5">
                    <label class="text-sm font-medium" for="dialog-plex-server">Plex Server</label>
                    <select
                      id="dialog-plex-server"
                      bind:value={plexSelectedServerUri}
                      class="w-full rounded-lg border border-border bg-background/40 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      onchange={() => applySelectedPlexServer(plexSelectedServerUri)}
                    >
                      {#each plexServers as server (server.machineIdentifier + server.uri)}
                        <option value={server.uri}>
                          {server.name}{server.owned ? '' : ' • Shared'}{server.local ? ' • Local' : ''}
                        </option>
                      {/each}
                    </select>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
        <div class="grid grid-cols-2 gap-4">
          <div class="space-y-1.5">
            <label class="text-sm font-medium" for="dialog-user">Username</label>
            <input
              id="dialog-user"
              type="text"
              bind:value={profileDraft.username}
              placeholder={profileDraft.serverType === 'local' ? 'Not used for local folders' : profileDraft.serverType === 'plex' ? 'Optional for Plex tokens' : profileDraft.serverType === 'octo_fiesta' ? 'Optional' : 'admin'}
              autocomplete="username"
              class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
              disabled={profileDraft.serverType === 'local'}
            />
          </div>
          <div class="space-y-1.5">
            <label class="text-sm font-medium" for="dialog-pass">
              {#if profileDraft.serverType === 'local' || profileDraft.serverType === 'octo_fiesta'}
                Password
              {:else if profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby'}
                API Key{profileDraft.id !== null ? ' (blank = keep current)' : ''}
              {:else if profileDraft.serverType === 'plex'}
                Plex Token{profileDraft.id !== null ? ' (blank = keep current)' : ''}
              {:else}
                {profileDraft.id !== null ? 'Password (blank = keep current)' : 'Password'}
              {/if}
            </label>
            <div class="relative">
              <input
                id="dialog-pass"
                type={showDraftPassword ? 'text' : 'password'}
                bind:value={profileDraft.password}
                placeholder={profileDraft.serverType === 'local' || profileDraft.serverType === 'octo_fiesta'
                  ? 'Not required'
                  : profileDraft.id !== null
                    ? 'Leave blank to keep current'
                    : (profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby')
                      ? 'API key'
                      : profileDraft.serverType === 'plex'
                        ? 'X-Plex-Token'
                      : 'Password'}
                autocomplete="current-password"
                class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 pr-10 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                disabled={profileDraft.serverType === 'local' || profileDraft.serverType === 'octo_fiesta'}
              />
              <button
                type="button"
                class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                onclick={() => { showDraftPassword = !showDraftPassword; }}
                aria-label={showDraftPassword ? 'Hide password' : 'Show password'}
              >
                {#if showDraftPassword}<EyeOff class="size-4" />{:else}<Eye class="size-4" />{/if}
              </button>
            </div>
          </div>
        </div>
      </div>
      </form>
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
        class="flex items-center gap-2 rounded-lg border border-border px-4 py-2 text-sm font-medium transition hover:bg-secondary disabled:opacity-50"
        type="button"
        onclick={testConnection}
        disabled={testingConnection || savingProfile || !profileDraft?.url}
      >
        {#if testingConnection}
          <Loader2 class="size-4 animate-spin" />
          Testing…
        {:else}
          <Wifi class="size-4" />
          Test
        {/if}
      </button>
      <button
        class="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90 disabled:opacity-60"
        type="submit"
        form="server-profile-form"
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
