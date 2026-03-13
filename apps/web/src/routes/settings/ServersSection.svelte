<script lang="ts">
  import { Server, Plus, Pencil, Trash2, CheckCircle, Eye, EyeOff, Save, Loader2 } from '@lucide/svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { toast } from 'svelte-sonner';
  import {
    activateProfile as activateProfileApi,
    createProfile,
    deleteProfile as deleteProfileApi,
    updateProfile,
    type ProfileDraftPayload,
    type ProfilePayload,
  } from '$lib/api';
  import { libraryRefresh } from '$lib/stores/settings';

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
    subsonic: 'Subsonic (Token Modern)',
    subsonic_legacy: 'Subsonic (Token Legacy)',
    jellyfin: 'Jellyfin',
    emby: 'Emby',
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

  $effect(() => { if (!dialogOpen) profileDraft = null; });

  function openAdd() {
    profileDraft = { id: null, name: '', url: '', username: '', password: '', serverType: 'subsonic' };
    showDraftPassword = false;
    dialogOpen = true;
  }

  function openEdit(p: Profile) {
    profileDraft = { id: p.id, name: p.name, url: p.url, username: p.username, password: '', serverType: p.serverType };
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
      dialogOpen = false;
      toast.success(isNew ? 'Server added' : 'Server updated');
      onHealthChange();
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
      libraryRefresh.update((n) => n + 1);
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
      <p class="mt-0.5 text-xs text-muted-foreground">Manage your Subsonic, Jellyfin, and Emby connections.</p>
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
      <p class="mt-1 text-xs text-muted-foreground/70">Add your first Navidrome, Subsonic, Jellyfin, or Emby server above.</p>
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

<Dialog.Root bind:open={dialogOpen}>
  <Dialog.Content class="sm:max-w-xl">
    <Dialog.Header>
      <Dialog.Title>{profileDraft?.id === null ? 'Add Server' : 'Edit Server'}</Dialog.Title>
      <Dialog.Description>Configure your server connection. For Jellyfin/Emby, use your API key as the password.</Dialog.Description>
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
            <label class="text-sm font-medium" for="dialog-server-type">Server Type</label>
            <select
              id="dialog-server-type"
              bind:value={profileDraft.serverType}
              class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="subsonic">Subsonic (Token Modern)</option>
              <option value="subsonic_legacy">Subsonic (Token Legacy)</option>
              <option value="jellyfin">Jellyfin</option>
              <option value="emby">Emby</option>
            </select>
          </div>
        </div>
        <div class="space-y-1.5">
          <label class="text-sm font-medium" for="dialog-url">Server URL</label>
          <input
            id="dialog-url"
            type="url"
            bind:value={profileDraft.url}
            placeholder={(profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby') ? 'http://localhost:8096' : 'http://localhost:4533'}
            class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
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
              {#if profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby'}
                API Key{profileDraft.id !== null ? ' (blank = keep current)' : ''}
              {:else}
                {profileDraft.id !== null ? 'Password (blank = keep current)' : 'Password'}
              {/if}
            </label>
            <div class="relative">
              <input
                id="dialog-pass"
                type={showDraftPassword ? 'text' : 'password'}
                bind:value={profileDraft.password}
                placeholder={profileDraft.id !== null ? 'Leave blank to keep current' : (profileDraft.serverType === 'jellyfin' || profileDraft.serverType === 'emby') ? 'API key' : 'Password'}
                autocomplete="current-password"
                class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 pr-10 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
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
