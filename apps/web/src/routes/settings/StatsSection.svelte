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
  import { fetchLibraryStats, fetchServiceHealth, type LibraryStatsPayload } from '$lib/api';
  import type { ServiceStatus } from '@player/shared/contracts';

  // Increment from outside to trigger a health + stats reload
  let { refreshKey = 0 }: { refreshKey?: number } = $props();

  let subsonicStatus = $state<ServiceStatus>('checking');
  let lastfmStatus = $state<ServiceStatus>('checking');
  let recheckingHealth = $state(false);

  let stats = $state<LibraryStatsPayload | null>(null);
  let statsLoading = $state(true);

  onMount(() => { loadAll(); });

  $effect(() => {
    // Re-run when refreshKey changes (after 0)
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

  function fmt(n: number | null | undefined): string {
    if (n === null || n === undefined) return '—';
    return n.toLocaleString();
  }
</script>

<!-- ── Service Status ─────────────────────────────────────────────────── -->
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
    {#each [{ label: 'Library', status: subsonicStatus }, { label: 'Last.fm', status: lastfmStatus }] as svc (svc.label)}
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

<!-- ── Library Stats ──────────────────────────────────────────────────── -->
<section>
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Library Stats</h2>
    <button
      class="flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs text-muted-foreground transition hover:bg-secondary hover:text-foreground disabled:opacity-50"
      onclick={refreshStats}
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
      { label: 'Playlist Tracks', value: stats?.totalPlaylistSongs, icon: Music2, color: 'text-violet-400' },
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
