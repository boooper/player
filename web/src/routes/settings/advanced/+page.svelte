<script lang="ts">
  import { Download, Gamepad2, Loader2 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import PlaybackSection from '../PlaybackSection.svelte';
  import DangerZoneSection from '../DangerZoneSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';
  import { checkForAppUpdate, installAppUpdate, type AppUpdateInfo } from '$lib/updater';

  const settings = useSettingsContext();
  let updateInfo = $state<AppUpdateInfo | null>(null);
  let updateChecking = $state(false);
  let updateInstalling = $state(false);
  let updaterUnavailable = $state(false);

  async function handleCheckForUpdates() {
    updateChecking = true;
    updaterUnavailable = false;
    try {
      updateInfo = await checkForAppUpdate();
      if (!updateInfo) {
        toast.success('You are already on the latest version');
      }
    } catch (error) {
      updaterUnavailable = true;
      toast.error(error instanceof Error ? error.message : String(error));
    } finally {
      updateChecking = false;
    }
  }

  async function handleInstallUpdate() {
    updateInstalling = true;
    try {
      const installed = await installAppUpdate();
      if (!installed) {
        updateInfo = null;
        toast.success('You are already on the latest version');
        return;
      }
      toast.success(`Update ${installed.version} installed. Relaunching...`);
      updateInfo = null;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to install update');
    } finally {
      updateInstalling = false;
    }
  }
</script>

<div class="space-y-8">
  {#if !settings.isMobile}
    <div class="page-section">
      <PlaybackSection
        bind:autostartEnabled={settings.autostartEnabled}
        bind:crossfadeSeconds={settings.crossfadeSeconds}
        bind:eqEnabled={settings.eqEnabled}
        bind:eqPreset={settings.eqPreset}
        bind:eqBands={settings.eqBands}
        showPlayback={false}
        showEqualizer={false}
      />
    </div>
  {/if}
  {#if !settings.isMobile}
  <section class="page-section rounded-xl border border-border/70 bg-card">
    <div class="border-b border-border/60 px-5 py-4">
      <div class="flex items-center gap-2">
        <Gamepad2 class="size-4 text-muted-foreground" />
        <h2 class="font-semibold">Discord</h2>
      </div>
      <p class="mt-0.5 text-xs text-muted-foreground">Desktop rich presence integration.</p>
    </div>
    <div class="px-5 py-5">
      <label class="flex cursor-pointer items-center gap-3" for="discord-rpc-toggle">
        <input
          id="discord-rpc-toggle"
          type="checkbox"
          class="sr-only"
          bind:checked={settings.discordRpcEnabled}
        />
        <div
          aria-hidden="true"
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {settings.discordRpcEnabled ? 'bg-primary' : 'bg-input'}"
        >
          <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {settings.discordRpcEnabled ? 'translate-x-4' : ''}"></span>
        </div>
        <div>
          <p class="text-sm font-medium">Enable Discord RPC</p>
          <p class="text-xs text-muted-foreground">Show the currently playing track in Discord rich presence.</p>
        </div>
      </label>
    </div>
  </section>
  <section class="page-section rounded-xl border border-border/70 bg-card">
    <div class="border-b border-border/60 px-5 py-4">
      <div class="flex items-center gap-2">
        <Download class="size-4 text-muted-foreground" />
        <h2 class="font-semibold">Updates</h2>
      </div>
      <p class="mt-0.5 text-xs text-muted-foreground">Check for signed desktop releases and install them in-app.</p>
    </div>
    <div class="px-5 py-5">
      <div class="flex items-center justify-between gap-4">
        <div class="min-w-0">
          <p class="text-sm font-medium">
            {#if updateInfo}
              Update {updateInfo.version} available
            {:else}
              Check for updates
            {/if}
          </p>
          <p class="text-xs text-muted-foreground">
            {#if updaterUnavailable}
              Updater is not configured in this build.
            {:else if !updateInfo}
              Looks for the latest Madrify release from GitHub Releases.
            {/if}
          </p>
        </div>
        {#if updateInfo}
          <button
            class="flex shrink-0 items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90 disabled:opacity-60"
            onclick={handleInstallUpdate}
            disabled={updateInstalling || updateChecking}
          >
            {#if updateInstalling}
              <Loader2 class="size-4 animate-spin" />
              Installing...
            {:else}
              <Download class="size-4" />
              Install {updateInfo.version}
            {/if}
          </button>
        {:else}
          <button
            class="flex shrink-0 items-center gap-2 rounded-lg border border-border px-4 py-2 text-sm font-medium transition hover:bg-border/20 disabled:opacity-60"
            onclick={handleCheckForUpdates}
            disabled={updateChecking || updateInstalling}
          >
            {#if updateChecking}
              <Loader2 class="size-4 animate-spin" />
              Checking...
            {:else}
              <Download class="size-4" />
              Check now
            {/if}
          </button>
        {/if}
      </div>
      {#if updateInfo?.body}
        <div class="mt-3 max-h-36 overflow-y-auto rounded-lg bg-muted/50 px-3 py-2.5">
          <p class="mb-1 text-xs font-medium text-muted-foreground">What's new</p>
          <ul class="space-y-0.5">
            {#each updateInfo.body.split('\n').filter(l => l.trim()) as line}
              <li class="text-xs text-foreground/80">{line.replace(/^-\s*/, '• ')}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  </section>
  {/if}
  <div class="page-section">
    <DangerZoneSection onCacheCleared={settings.refreshStats} onCleared={settings.handleCleared} />
  </div>
</div>
