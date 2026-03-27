<script lang="ts">
  import { onMount } from 'svelte';
  import { BarChart2, Music2, Mic2, Tag, Clock, PlayCircle, SkipForward, CheckCircle, RefreshCw } from '@lucide/svelte';
  import { getListeningProfile, type ListeningProfile, type SongStat, type ArtistStat, type GenreStat } from '$lib/servers/play-history';
  import { playNextInQueue, appendToQueue } from '$lib/stores/player';
  import { searchSongs, type Song } from '$lib/servers';
  import { initials } from '$lib/utils';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import { goto } from '$app/navigation';

  type Tab = 'songs' | 'artists' | 'genres';

  let profile = $state<ListeningProfile | null>(null);
  let loading = $state(true);
  let activeTab = $state<Tab>('songs');

  async function refresh() {
    loading = true;
    profile = await getListeningProfile();
    loading = false;
  }

  onMount(refresh);

  // ── Formatting helpers ──────────────────────────────────────────────────────
  function formatListenTime(secs: number): string {
    if (secs < 60) return `${Math.round(secs)}s`;
    const mins = secs / 60;
    if (mins < 60) return `${Math.floor(mins)}m`;
    const hours = mins / 60;
    if (hours < 24) return `${Math.floor(hours)}h ${Math.floor(mins % 60)}m`;
    const days = hours / 24;
    return `${Math.floor(days)}d ${Math.floor(hours % 24)}h`;
  }

  function skipRate(stat: SongStat | ArtistStat): number {
    return stat.playCount > 0 ? Math.round((stat.skipCount / stat.playCount) * 100) : 0;
  }

  function completionPct(stat: SongStat | ArtistStat): number {
    return Math.round((stat.avgCompletion ?? 0) * 100);
  }

  function skipColor(rate: number): string {
    if (rate < 15) return 'bg-emerald-500';
    if (rate < 35) return 'bg-yellow-500';
    return 'bg-rose-500';
  }


  // ── Song lookup for context menu play action ────────────────────────────────
  async function playSong(stat: SongStat) {
    const results = await searchSongs(`${stat.artist} ${stat.title}`, 5).catch(() => [] as Song[]);
    const match = results.find((s) => s.id === stat.songId) ?? results[0];
    if (match) {
      appendToQueue([match]);
      playNextInQueue(match);
    }
  }

  // ── Summary stats ───────────────────────────────────────────────────────────
  const topGenre = $derived(profile?.topGenres[0]?.genre ?? null);
</script>

<div class="pb-16">

  {#if loading}
    <!-- Skeleton -->
    <div class="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
      {#each [1,2,3,4] as _}
        <div class="rounded-xl border border-border/40 bg-card p-4 space-y-2">
          <div class="h-3 w-16 rounded bg-muted animate-pulse"></div>
          <div class="h-6 w-12 rounded bg-muted animate-pulse"></div>
        </div>
      {/each}
    </div>
    <div class="space-y-2">
      {#each [1,2,3,4,5] as _}
        <div class="flex items-center gap-3 rounded-xl border border-border/40 bg-card p-3">
          <div class="size-10 shrink-0 rounded-lg bg-muted animate-pulse"></div>
          <div class="flex-1 space-y-1.5">
            <div class="h-3 w-2/5 rounded bg-muted animate-pulse"></div>
            <div class="h-2.5 w-1/4 rounded bg-muted animate-pulse"></div>
          </div>
        </div>
      {/each}
    </div>

  {:else if !profile || profile.totalPlays === 0}
    <div class="flex flex-col items-center gap-3 py-20 text-center">
      <BarChart2 class="size-12 text-muted-foreground/30" />
      <p class="text-base font-medium text-muted-foreground">No listening history yet</p>
      <p class="text-sm text-muted-foreground/60">Start playing music — your stats will appear here.</p>
    </div>

  {:else}
    <!-- Overview cards -->
    <div class="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
      <div class="rounded-xl border border-border/40 bg-card p-4">
        <p class="mb-1 text-xs font-medium text-muted-foreground">Total Plays</p>
        <p class="text-2xl font-bold tabular-nums">{profile.totalPlays.toLocaleString()}</p>
      </div>
      <div class="rounded-xl border border-border/40 bg-card p-4">
        <p class="mb-1 text-xs font-medium text-muted-foreground">Listen Time</p>
        <p class="text-2xl font-bold tabular-nums">{formatListenTime(profile.totalListenSecs)}</p>
      </div>
      <div class="rounded-xl border border-border/40 bg-card p-4">
        <p class="mb-1 text-xs font-medium text-muted-foreground">Artists Played</p>
        <p class="text-2xl font-bold tabular-nums">{profile.uniqueArtists.toLocaleString()}</p>
      </div>
      <div class="rounded-xl border border-border/40 bg-card p-4">
        <p class="mb-1 text-xs font-medium text-muted-foreground">Top Genre</p>
        <p class="truncate text-2xl font-bold">{topGenre ?? '—'}</p>
      </div>
    </div>

    <!-- Tabs + refresh -->
    <div class="mb-5 flex items-center gap-2">
    <button
      class="ml-auto flex items-center gap-1.5 rounded-lg px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
      onclick={refresh}
      title="Refresh stats"
    >
      <RefreshCw class="size-3.5" />
      Refresh
    </button>
    </div>
    <div class="mb-5 flex gap-1 rounded-xl border border-border/40 bg-card p-1">
      {#each ([['songs', Music2, 'Top Songs'], ['artists', Mic2, 'Top Artists'], ['genres', Tag, 'Top Genres']] as const) as [id, Icon, label]}
        <button
          class="flex flex-1 items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors
            {activeTab === id ? 'bg-background shadow-sm text-foreground' : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => { activeTab = id; }}
        >
          <Icon class="size-3.5" />
          {label}
        </button>
      {/each}
    </div>

    <!-- Top Songs -->
    {#if activeTab === 'songs'}
      {#if profile.topSongs.length === 0}
        <p class="py-10 text-center text-sm text-muted-foreground">No song data yet.</p>
      {:else}
        <div class="space-y-1">
          {#each profile.topSongs as stat, i (stat.songId)}
            {@const skip = skipRate(stat)}
            {@const completion = completionPct(stat)}
            <SongContextMenu
              song={{ id: stat.songId, title: stat.title, artist: stat.artist, album: stat.album ?? '', albumId: '', coverArtUrl: '', coverArt: null, streamUrl: '', duration: 0 }}
              onplay={() => playSong(stat)}
              triggerClass="block w-full"
            >
              <div class="group flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-accent">
                <!-- Rank -->
                <span class="w-5 shrink-0 text-right text-xs font-semibold tabular-nums text-muted-foreground/50">{i + 1}</span>

                <!-- Art -->
                {#if stat.coverArtUrl}
                  <img src={stat.coverArtUrl} alt={stat.title} class="size-10 shrink-0 rounded-lg object-cover" />
                {:else}
                  <div class="grid size-10 shrink-0 place-items-center rounded-lg bg-secondary text-xs font-bold">{initials(stat.title)}</div>
                {/if}

                <!-- Title / Artist -->
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium leading-tight">{stat.title}</p>
                  <button
                    class="truncate text-xs text-muted-foreground hover:text-foreground hover:underline transition-colors text-left"
                    onclick={(e) => { e.stopPropagation(); goto(`/artist/${encodeURIComponent(stat.artist)}`); }}
                  >{stat.artist}</button>
                </div>

                <!-- Play count -->
                <div class="hidden sm:flex flex-col items-center gap-0.5 w-12 shrink-0">
                  <span class="flex items-center gap-1 text-xs font-semibold tabular-nums">
                    <PlayCircle class="size-3 text-muted-foreground" />{stat.playCount}
                  </span>
                  <span class="text-[10px] text-muted-foreground">plays</span>
                </div>

                <!-- Listen time -->
                <div class="hidden sm:flex flex-col items-center gap-0.5 w-14 shrink-0">
                  <span class="flex items-center gap-1 text-xs font-semibold tabular-nums">
                    <Clock class="size-3 text-muted-foreground" />{formatListenTime(stat.totalListenSecs)}
                  </span>
                  <span class="text-[10px] text-muted-foreground">listened</span>
                </div>

                <!-- Skip rate + completion bar -->
                <div class="hidden md:block w-28 shrink-0 space-y-1">
                  <div class="flex items-center justify-between text-[10px] text-muted-foreground">
                    <span class="flex items-center gap-1"><SkipForward class="size-3" />{skip}% skip</span>
                    <span class="flex items-center gap-1"><CheckCircle class="size-3" />{completion}%</span>
                  </div>
                  <div class="h-1 w-full overflow-hidden rounded-full bg-muted">
                    <div class="h-full rounded-full bg-primary/60 transition-all" style="width:{completion}%"></div>
                  </div>
                </div>
              </div>
            </SongContextMenu>
          {/each}
        </div>
      {/if}

    <!-- Top Artists -->
    {:else if activeTab === 'artists'}
      {#if profile.topArtists.length === 0}
        <p class="py-10 text-center text-sm text-muted-foreground">No artist data yet.</p>
      {:else}
        <div class="space-y-1">
          {#each profile.topArtists as stat, i (stat.artist)}
            {@const skip = skipRate(stat)}
            {@const completion = completionPct(stat)}
            <button
              class="group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors hover:bg-accent"
              onclick={() => goto(`/artist/${encodeURIComponent(stat.artist)}`)}
            >
              <span class="w-5 shrink-0 text-right text-xs font-semibold tabular-nums text-muted-foreground/50">{i + 1}</span>

              {#if stat.coverArtUrl}
                <img src={stat.coverArtUrl} alt={stat.artist} class="size-10 shrink-0 rounded-full object-cover" />
              {:else}
                <div class="grid size-10 shrink-0 place-items-center rounded-full bg-secondary text-xs font-bold">{initials(stat.artist)}</div>
              {/if}

              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">{stat.artist}</p>
                <p class="text-xs text-muted-foreground">{stat.playCount} plays · {formatListenTime(stat.totalListenSecs)}</p>
              </div>

              <!-- Completion / skip bar -->
              <div class="hidden sm:block w-36 shrink-0 space-y-1">
                <div class="flex items-center justify-between text-[10px] text-muted-foreground">
                  <span>{completion}% avg completion</span>
                  <span class="flex items-center gap-0.5">
                    <span class="inline-block size-1.5 rounded-full {skipColor(skip)}"></span>
                    {skip}% skip
                  </span>
                </div>
                <div class="h-1 w-full overflow-hidden rounded-full bg-muted">
                  <div class="h-full rounded-full bg-primary/60 transition-all" style="width:{completion}%"></div>
                </div>
              </div>
            </button>
          {/each}
        </div>
      {/if}

    <!-- Top Genres -->
    {:else if activeTab === 'genres'}
      {#if profile.topGenres.length === 0}
        <p class="py-10 text-center text-sm text-muted-foreground">No genre data yet — genre tagging requires song metadata.</p>
      {:else}
        {@const maxPlays = profile.topGenres[0]?.playCount ?? 1}
        <div class="space-y-3">
          {#each profile.topGenres as stat, i (stat.genre)}
            <div class="rounded-xl border border-border/40 bg-card p-4">
              <div class="mb-2 flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span class="text-xs font-semibold tabular-nums text-muted-foreground/50">#{i + 1}</span>
                  <span class="text-sm font-semibold">{stat.genre}</span>
                </div>
                <div class="flex items-center gap-3 text-xs text-muted-foreground">
                  <span class="flex items-center gap-1"><PlayCircle class="size-3" />{stat.playCount} plays</span>
                  <span class="flex items-center gap-1"><CheckCircle class="size-3" />{Math.round(stat.avgCompletion * 100)}% avg</span>
                </div>
              </div>
              <!-- Proportional bar -->
              <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary/70 transition-all"
                  style="width:{Math.round((stat.playCount / maxPlays) * 100)}%"
                ></div>
              </div>
              <!-- Mini skip/full breakdown -->
              <div class="mt-1.5 flex gap-3 text-[10px] text-muted-foreground">
                <span class="flex items-center gap-1"><CheckCircle class="size-3 text-emerald-500" />{stat.fullPlayCount} full plays</span>
                <span class="flex items-center gap-1"><SkipForward class="size-3 text-rose-400" />{stat.skipCount} skipped</span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}

  {/if}
</div>
