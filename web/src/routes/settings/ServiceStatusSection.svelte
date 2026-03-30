<script lang="ts">
  import {
    Server,
    RefreshCw,
    CheckCircle2,
    XCircle,
    AlertCircle,
    Loader2,
    Heart,
    Star,
    ListMusic,
    Music2,
  } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { fetchLibraryStats, fetchServiceHealth, type LibraryStatsPayload } from '$lib/servers';
  import type { ServiceStatus } from '@player/shared/contracts';

  let { refreshKey = 0 }: { refreshKey?: number } = $props();

  let subsonicStatus = $state<ServiceStatus>('checking');
  let lastfmStatus = $state<ServiceStatus>('checking');
  let recheckingHealth = $state(false);

  let stats = $state<LibraryStatsPayload | null>(null);
  let statsLoading = $state(true);

  onMount(() => { loadAll(); });

  $effect(() => {
    if (refreshKey > 0) loadAll();
  });

  async function loadAll() {
    await Promise.all([recheckConnections(), refreshStats()]);
  }

  async function recheckConnections() {
    recheckingHealth = true;
    subsonicStatus = 'checking';
    lastfmStatus = 'checking';
    try {
      const payload = await fetchServiceHealth();
      subsonicStatus = payload?.subsonic ?? 'offline';
      lastfmStatus = payload?.lastfm ?? 'offline';
    } catch {
      subsonicStatus = 'offline';
      lastfmStatus = 'offline';
    } finally {
      recheckingHealth = false;
    }
  }

  async function refreshStats() {
    statsLoading = true;
    try {
      stats = await fetchLibraryStats();
    } catch {
      stats = null;
    } finally {
      statsLoading = false;
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

  function statusLabel(status: ServiceStatus): string {
    if (status === 'online') return 'Connected';
    if (status === 'checking') return 'Checking…';
    if (status === 'missing') return 'Not configured';
    return 'Unreachable';
  }

  function fmt(n: number | null | undefined): string {
    if (n === null || n === undefined) return '—';
    return n.toLocaleString();
  }
</script>

<section class="rounded-xl border border-border/70 bg-card">
  <!-- Header -->
  <div class="flex items-center justify-between border-b border-border/60 px-5 py-4">
    <div>
      <div class="flex items-center gap-2">
        <Server class="size-4 text-muted-foreground" />
        <h2 class="font-semibold">Status & Stats</h2>
      </div>
      <p class="mt-0.5 text-xs text-muted-foreground">Live connection health and library counts.</p>
    </div>
    <button
      class="flex items-center gap-1.5 rounded-md border border-border/60 px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition hover:bg-secondary hover:text-foreground disabled:opacity-50"
      onclick={loadAll}
      disabled={recheckingHealth || statsLoading}
    >
      <RefreshCw class="size-3.5 {recheckingHealth || statsLoading ? 'animate-spin' : ''}" />
      Refresh
    </button>
  </div>

  <div class="divide-y divide-border/40">
    <!-- Service connections -->
    <div class="grid grid-cols-2 gap-3 px-5 py-4">
      {#each [{ label: 'Library', status: subsonicStatus }, { label: 'Last.fm', status: lastfmStatus }] as svc (svc.label)}
        {@const Icon = statusIcon(svc.status)}
        <div class="flex items-center gap-3 rounded-lg border border-border/50 bg-secondary/20 px-4 py-3">
          <Server class="size-4 shrink-0 text-muted-foreground" />
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium">{svc.label}</p>
            <p class="text-xs text-muted-foreground">{statusLabel(svc.status)}</p>
          </div>
          <Icon class="size-4 shrink-0 {statusClass(svc.status)} {svc.status === 'checking' ? 'animate-spin' : ''}" />
        </div>
      {/each}
    </div>

    <!-- Library stats -->
    <div class="px-5 py-4">
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {#each [
          { label: 'Saved Artists', value: stats?.likedArtists, icon: Heart, color: 'text-rose-400' },
          { label: 'Starred Songs', value: stats?.starredSongs, icon: Star, color: 'text-amber-400' },
          { label: 'Playlists', value: stats?.playlistCount, icon: ListMusic, color: 'text-sky-400' },
          { label: 'Playlist Tracks', value: stats?.totalPlaylistSongs, icon: Music2, color: 'text-violet-400' },
        ] as card (card.label)}
          {@const Icon = card.icon}
          <div class="flex flex-col gap-2 rounded-lg border border-border/50 bg-secondary/20 px-4 py-3.5">
            <Icon class="size-4 {card.color}" />
            <div>
              {#if statsLoading}
                <div class="h-6 w-10 animate-pulse rounded bg-secondary"></div>
              {:else}
                <p class="text-xl font-bold tabular-nums">{fmt(card.value)}</p>
              {/if}
              <p class="text-xs text-muted-foreground">{card.label}</p>
            </div>
          </div>
        {/each}
      </div>
      {#if stats && stats.starredSongs === null}
        <p class="mt-2 text-xs text-muted-foreground">Some stats are unavailable — Subsonic may not be connected.</p>
      {/if}
    </div>
  </div>
</section>
