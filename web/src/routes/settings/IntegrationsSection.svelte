<script lang="ts">
  import { Music2, Eye, EyeOff, User, Unlink, Link, ExternalLink, Loader2 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { openUrl } from '$lib/tauri';
  import { lfmBeginAuth, lfmCompleteAuth, lfmDisconnect } from '$lib/servers';
  import * as Select from '$lib/components/ui/select';

  const META_LABELS: Record<string, string> = {
    local: 'Local library first',
    both: 'Both (Last.fm + TheAudioDB)',
    lastfm: 'Last.fm only',
    audiodb: 'TheAudioDB only',
  };
  const REC_LABELS: Record<string, string> = { lastfm: 'Last.fm', listenbrainz: 'ListenBrainz' };
  const LYRICS_LABELS: Record<string, string> = {
    auto: 'Automatic',
    musixmatch: 'Musixmatch',
    lrclib: 'LRCLIB',
  };

  let {
    isMobile = false,
    lastFmApiKey = $bindable(''),
    lastFmSharedSecret = $bindable(''),
    lastFmSharedSecretConfigured = $bindable(false),
    lfmConnected = $bindable(false),
    lfmUsername = $bindable(''),
    listenBrainzUsername = $bindable(''),
    listenBrainzToken = $bindable(''),
    listenBrainzTokenConfigured = $bindable(false),
    metadataProvider = $bindable('both'),
    recommendationProvider = $bindable('lastfm'),
    lyricsProvider = $bindable('auto'),
    onHealthChange,
  }: {
    isMobile?: boolean;
    lastFmApiKey: string;
    lastFmSharedSecret: string;
    lastFmSharedSecretConfigured: boolean;
    lfmConnected: boolean;
    lfmUsername: string;
    listenBrainzUsername: string;
    listenBrainzToken: string;
    listenBrainzTokenConfigured: boolean;
    metadataProvider: string;
    recommendationProvider: string;
    lyricsProvider: string;
    onHealthChange: () => void;
  } = $props();

  let showApiKey = $state(false);
  let showSharedSecret = $state(false);
  let lfmAuthState = $state<'idle' | 'pending' | 'completing'>('idle');
  let lfmPendingToken = $state('');
  let lfmConnecting = $state(false);
  let lfmDisconnecting = $state(false);

  $effect(() => {
    backendSettings.update((s) => ({ ...s, lastFmConnected: lfmConnected, lastFmUsername: lfmUsername }));
  });
  $effect(() => {
    backendSettings.update((s) => ({ ...s, lyricsProvider }));
  });

  async function connectLastFm() {
    lfmConnecting = true;
    try {
      const { token, authUrl } = await lfmBeginAuth();
      lfmPendingToken = token;
      lfmAuthState = 'pending';
      await openUrl(authUrl);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err) || 'Failed to start Last.fm auth');
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
      onHealthChange();
    } catch (err) {
      lfmAuthState = 'pending';
      toast.error(err instanceof Error ? err.message : String(err) || 'Authorization failed — make sure you approved the app on Last.fm');
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
</script>

<section class="rounded-xl border border-border/70 bg-card">
  <div class="border-b border-border/60 px-5 py-4">
    <div class="flex items-center gap-2">
      <Music2 class="size-4 text-muted-foreground" />
      <h2 class="font-semibold">Last.fm</h2>
    </div>
    <p class="mt-0.5 text-xs text-muted-foreground">
      Powers recommendations, mixes, and scrobbling. Get a free API key at
      <a href="https://www.last.fm/api/account/create" target="_blank" rel="noopener noreferrer" class="text-primary underline-offset-2 hover:underline">last.fm/api</a>.
    </p>
  </div>

  {#if !isMobile}
  <!-- svelte-ignore a11y_no_redundant_roles -->
  <form class="divide-y divide-border/40 px-5" autocomplete="off" onsubmit={(e) => e.preventDefault()}>
    <input type="text" name="username" autocomplete="username" aria-hidden="true" class="sr-only" tabindex="-1" />

    <!-- API Key -->
    <div class="flex items-center gap-4 py-4">
      <label class="w-32 shrink-0 text-sm font-medium" for="lfm-key">API Key</label>
      <div class="relative flex-1">
        <input
          id="lfm-key"
          type={showApiKey ? 'text' : 'password'}
          bind:value={lastFmApiKey}
          placeholder="Paste your Last.fm API key"
          autocomplete="new-password"
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
    <div class="flex items-center gap-4 py-4">
      <div class="w-32 shrink-0">
        <p class="text-sm font-medium">Shared Secret</p>
        <p class="text-[11px] text-muted-foreground">For scrobbling</p>
      </div>
      <div class="relative flex-1">
        <input
          id="lfm-secret"
          type={showSharedSecret ? 'text' : 'password'}
          bind:value={lastFmSharedSecret}
          placeholder={lastFmSharedSecretConfigured ? 'Already configured — enter to replace' : 'Paste your Last.fm shared secret'}
          autocomplete="new-password"
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

    <!-- Account -->
    <div class="flex items-center gap-4 py-4">
      <p class="w-32 shrink-0 text-sm font-medium">Account</p>
      {#if lfmConnected}
        <div class="flex flex-1 items-center justify-between gap-3">
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
            {#if lfmDisconnecting}<Loader2 class="size-3.5 animate-spin" />{:else}<Unlink class="size-3.5" />{/if}
            Disconnect
          </button>
        </div>

      {:else if lfmAuthState === 'pending' || lfmAuthState === 'completing'}
        <div class="flex-1 space-y-2">
          <div class="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-600 dark:text-amber-400">
            <ExternalLink class="mt-0.5 size-3.5 shrink-0" />
            <span>Authorize the app in your browser, then click below.</span>
          </div>
          <div class="flex gap-2">
            <button
              class="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground transition hover:bg-primary/90 disabled:opacity-50"
              onclick={completeLastFmAuth}
              disabled={lfmAuthState === 'completing'}
            >
              {#if lfmAuthState === 'completing'}<Loader2 class="size-4 animate-spin" />Verifying…
              {:else}<Link class="size-4" />I've Authorized — Connect{/if}
            </button>
            <button
              class="rounded-md border border-border/70 px-3 py-2 text-sm text-muted-foreground transition hover:bg-secondary"
              onclick={cancelLastFmAuth}
              disabled={lfmAuthState === 'completing'}
            >Cancel</button>
          </div>
        </div>

      {:else}
        <div class="flex flex-1 items-center justify-between gap-3">
          <p class="text-xs text-muted-foreground">Link your account to enable scrobbling and personalised recommendations.</p>
          <button
            class="flex shrink-0 items-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground transition hover:bg-primary/90 disabled:opacity-50"
            onclick={connectLastFm}
            disabled={lfmConnecting || (!lastFmSharedSecretConfigured && !lastFmSharedSecret.trim())}
          >
            {#if lfmConnecting}<Loader2 class="size-4 animate-spin" />Opening…
            {:else}<Link class="size-4" />Connect{/if}
          </button>
        </div>
        {#if !lastFmSharedSecretConfigured && !lastFmSharedSecret.trim()}
          <p class="text-xs text-amber-500">Save your shared secret before connecting.</p>
        {/if}
      {/if}
    </div>
  </form>
  {/if}

  <!-- Providers -->
  <div class="border-t border-border/60 px-5 py-4">
    <p class="mb-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Providers</p>
    <div class="grid grid-cols-3 gap-4">
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Metadata</p>
        <Select.Root type="single" bind:value={metadataProvider}>
          <Select.Trigger class="w-full">{META_LABELS[metadataProvider] ?? metadataProvider}</Select.Trigger>
          <Select.Content>
            <Select.Item value="local" label="Local library first" />
            <Select.Item value="both" label="Both (Last.fm + TheAudioDB)" />
            <Select.Item value="lastfm" label="Last.fm only" />
            <Select.Item value="audiodb" label="TheAudioDB only" />
          </Select.Content>
        </Select.Root>
      </div>
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Recommendations</p>
        <Select.Root type="single" bind:value={recommendationProvider}>
          <Select.Trigger class="w-full">{REC_LABELS[recommendationProvider] ?? recommendationProvider}</Select.Trigger>
          <Select.Content>
            <Select.Item value="lastfm" label="Last.fm" />
            <Select.Item value="listenbrainz" label="ListenBrainz" />
          </Select.Content>
        </Select.Root>
      </div>
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Lyrics</p>
        <Select.Root type="single" bind:value={lyricsProvider}>
          <Select.Trigger class="w-full">{LYRICS_LABELS[lyricsProvider] ?? lyricsProvider}</Select.Trigger>
          <Select.Content>
            <Select.Item value="auto" label="Automatic" />
            <Select.Item value="musixmatch" label="Musixmatch" />
            <Select.Item value="lrclib" label="LRCLIB" />
          </Select.Content>
        </Select.Root>
      </div>
    </div>
  </div>

  {#if recommendationProvider === 'listenbrainz'}
  <form class="border-t border-border/60 px-5 py-4" autocomplete="off" onsubmit={(e) => e.preventDefault()}>
    <p class="mb-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">ListenBrainz</p>
    <div class="grid grid-cols-2 gap-4">
      <div class="space-y-1.5">
        <label class="text-sm font-medium" for="lbz-username">Username</label>
        <input
          id="lbz-username"
          type="text"
          bind:value={listenBrainzUsername}
          placeholder="Your listenbrainz.org username"
          autocomplete="username"
          class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium" for="lbz-token">
          User Token
          <span class="ml-1 text-xs font-normal text-muted-foreground">for scrobbling</span>
        </label>
        <input
          id="lbz-token"
          type="password"
          bind:value={listenBrainzToken}
          placeholder={listenBrainzTokenConfigured ? 'Already configured — enter to replace' : 'Paste your ListenBrainz user token'}
          autocomplete="new-password"
          class="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm placeholder:font-sans placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>
    </div>
  </form>
  {/if}
</section>
