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

  async function playSong(stat: SongStat) {
    const results = await searchSongs(`${stat.artist} ${stat.title}`, 5).catch(() => [] as Song[]);
    const match = results.find((s) => s.id === stat.songId) ?? results[0];
    if (match) {
      appendToQueue([match]);
      playNextInQueue(match);
    }
  }

  const topGenre = $derived(profile?.topGenres[0]?.genre ?? null);
</script>

<div class="space-y-6 pb-16">

  {#if loading}
    <!-- Skeleton -->
    <div class="rounded-xl border border-border/70 bg-card">
      <div class="border-b border-border/60 px-5 py-4">
        <div class="h-4 w-24 animate-pulse rounded bg-secondary"></div>
      </div>
      <div class="grid grid-cols-2 gap-3 p-5 sm:grid-cols-4">
        {#each [1,2,3,4] as _}
          <div class="space-y-2 rounded-lg border border-border/50 bg-secondary/20 px-4 py-3.5">
            <div class="h-3 w-16 animate-pulse rounded bg-secondary"></div>
            <div class="h-6 w-12 animate-pulse rounded bg-secondary"></div>
          </div>
        {/each}
      </div>
    </div>
    <div class="rounded-xl border border-border/70 bg-card">
      <div class="border-b border-border/60 px-5 py-4">
        <div class="h-4 w-32 animate-pulse rounded bg-secondary"></div>
      </div>
      <div class="space-y-1 p-5">
        {#each [1,2,3,4,5] as _}
          <div class="flex items-center gap-3 rounded-lg p-2.5">
            <div class="size-10 shrink-0 animate-pulse rounded-lg bg-secondary"></div>
            <div class="flex-1 space-y-1.5">
              <div class="h-3 w-2/5 animate-pulse rounded bg-secondary"></div>
              <div class="h-2.5 w-1/4 animate-pulse rounded bg-secondary"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>

  {:else if !profile || profile.totalPlays === 0}
    <div class="flex flex-col items-center gap-3 py-20 text-center">
      <BarChart2 class="size-12 text-muted-foreground/30" />
      <p class="text-base font-medium text-muted-foreground">No listening history yet</p>
      <p class="text-sm text-muted-foreground/60">Start playing music — your stats will appear here.</p>
    </div>

  {:else}
    <!-- Overview -->
    <section class="rounded-xl border border-border/70 bg-card">
      <div class="border-b border-border/60 px-5 py-4">
        <h2 class="font-semibold">Overview</h2>
        <p class="mt-0.5 text-xs text-muted-foreground">Lifetime listening summary.</p>
      </div>
      <div class="grid grid-cols-2 gap-3 p-5 sm:grid-cols-4">
        {#each [
          { label: 'Total Plays', value: profile.totalPlays.toLocaleString() },
          { label: 'Listen Time', value: formatListenTime(profile.totalListenSecs) },
          { label: 'Artists Played', value: profile.uniqueArtists.toLocaleString() },
          { label: 'Top Genre', value: topGenre ?? '—' },
        ] as card (card.label)}
          <div class="rounded-lg border border-border/50 bg-secondary/20 px-4 py-3.5">
            <p class="mb-1 text-xs text-muted-foreground">{card.label}</p>
            <p class="truncate text-xl font-bold tabular-nums">{card.value}</p>
          </div>
        {/each}
      </div>
    </section>

    <!-- Tabbed history -->
    <section class="rounded-xl border border-border/70 bg-card">
      <div class="flex items-center justify-between border-b border-border/60 px-5 py-4">
        <div>
          <h2 class="font-semibold">Listening History</h2>
          <p class="mt-0.5 text-xs text-muted-foreground">Your most played songs, artists, and genres.</p>
        </div>
        <button
          class="flex items-center gap-1.5 rounded-md border border-border/60 px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition hover:bg-secondary hover:text-foreground"
          onclick={refresh}
        >
          <RefreshCw class="size-3.5" />
          Refresh
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex gap-1 border-b border-border/60 px-5 pt-3">
        {#each ([['songs', Music2, 'Top Songs'], ['artists', Mic2, 'Top Artists'], ['genres', Tag, 'Top Genres']] as const) as [id, Icon, label]}
          <button
            class="flex items-center gap-1.5 rounded-t-lg px-4 py-2 text-sm font-medium transition-colors
              {activeTab === id
                ? 'border border-b-0 border-border/60 bg-card text-foreground -mb-px'
                : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => { activeTab = id; }}
          >
            <Icon class="size-3.5" />
            {label}
          </button>
        {/each}
      </div>

      <!-- Tab content -->
      <div class="px-5 py-4">

        {#if activeTab === 'songs'}
          {#if profile.topSongs.length === 0}
            <p class="py-10 text-center text-sm text-muted-foreground">No song data yet.</p>
          {:else}
            <div class="space-y-0.5">
              {#each profile.topSongs as stat, i (stat.songId)}
                {@const skip = skipRate(stat)}
                {@const completion = completionPct(stat)}
                <SongContextMenu
                  song={{ id: stat.songId, title: stat.title, artist: stat.artist, album: stat.album ?? '', albumId: '', coverArtUrl: '', coverArt: null, streamUrl: '', duration: 0 }}
                  onplay={() => playSong(stat)}
                  triggerClass="block w-full"
                >
                  <div class="group flex items-center gap-3 rounded-lg px-3 py-2.5 transition-colors hover:bg-accent">
                    <span class="w-5 shrink-0 text-right text-xs font-semibold tabular-nums text-muted-foreground/50">{i + 1}</span>
                    {#if stat.coverArtUrl}
                      <img src={stat.coverArtUrl} alt={stat.title} class="size-10 shrink-0 rounded-lg object-cover" />
                    {:else}
                      <div class="grid size-10 shrink-0 place-items-center rounded-lg bg-secondary text-xs font-bold">{initials(stat.title)}</div>
                    {/if}
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm font-medium leading-tight">{stat.title}</p>
                      <button
                        class="truncate text-xs text-muted-foreground transition-colors hover:text-foreground hover:underline text-left"
                        onclick={(e) => { e.stopPropagation(); goto(`/artist/${encodeURIComponent(stat.artist)}`); }}
                      >{stat.artist}</button>
                    </div>
                    <div class="hidden sm:flex flex-col items-center gap-0.5 w-12 shrink-0">
                      <span class="flex items-center gap-1 text-xs font-semibold tabular-nums">
                        <PlayCircle class="size-3 text-muted-foreground" />{stat.playCount}
                      </span>
                      <span class="text-[10px] text-muted-foreground">plays</span>
                    </div>
                    <div class="hidden sm:flex flex-col items-center gap-0.5 w-14 shrink-0">
                      <span class="flex items-center gap-1 text-xs font-semibold tabular-nums">
                        <Clock class="size-3 text-muted-foreground" />{formatListenTime(stat.totalListenSecs)}
                      </span>
                      <span class="text-[10px] text-muted-foreground">listened</span>
                    </div>
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

        {:else if activeTab === 'artists'}
          {#if profile.topArtists.length === 0}
            <p class="py-10 text-center text-sm text-muted-foreground">No artist data yet.</p>
          {:else}
            <div class="space-y-0.5">
              {#each profile.topArtists as stat, i (stat.artist)}
                {@const skip = skipRate(stat)}
                {@const completion = completionPct(stat)}
                <button
                  class="group flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors hover:bg-accent"
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

        {:else if activeTab === 'genres'}
          {#if profile.topGenres.length === 0}
            <p class="py-10 text-center text-sm text-muted-foreground">No genre data yet — genre tagging requires song metadata.</p>
          {:else}
            {@const maxPlays = profile.topGenres[0]?.playCount ?? 1}
            <div class="space-y-2">
              {#each profile.topGenres as stat, i (stat.genre)}
                <div class="rounded-lg border border-border/50 bg-secondary/20 p-4">
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
                  <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full bg-primary/70 transition-all"
                      style="width:{Math.round((stat.playCount / maxPlays) * 100)}%"
                    ></div>
                  </div>
                  <div class="mt-1.5 flex gap-3 text-[10px] text-muted-foreground">
                    <span class="flex items-center gap-1"><CheckCircle class="size-3 text-emerald-500" />{stat.fullPlayCount} full plays</span>
                    <span class="flex items-center gap-1"><SkipForward class="size-3 text-rose-400" />{stat.skipCount} skipped</span>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}

      </div>
    </section>
  {/if}

</div>
