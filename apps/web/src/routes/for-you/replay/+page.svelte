<script lang="ts">
  import { goto } from '$app/navigation';
  import { Play, Pause, Save, RefreshCw } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';

  import type { Song } from '$lib/servers';
  import { searchSongs, createPlaylist } from '$lib/servers';
  import { getListeningProfile, type SongStat } from '$lib/servers/play-history';
  import {
    playQueue, focusTrack, playingFrom,
    isPlaying, togglePlayRequest, queue, currentIndex
  } from '$lib/stores/player';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import { formatClockDuration } from '$lib/utils';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';

  const HREF = '/for-you/replay';

  let songs = $state<Song[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let loadVersion = 0;

  const active = $derived($playingFrom.href === HREF);
  const currentTrackId = $derived($queue[$currentIndex]?.id ?? '');

  function norm(s: string) { return s.trim().toLowerCase(); }

  function totalDuration(): string {
    return formatClockDuration(songs.reduce((acc, s) => acc + (s.duration ?? 0), 0));
  }

  async function resolveTopSongs(topSongs: SongStat[], limit = 20): Promise<Song[]> {
    const results: Song[] = [];
    const seen = new Set<string>();
    for (const stat of topSongs.slice(0, limit * 2)) {
      if (results.length >= limit) break;
      try {
        const hits = await searchSongs(`${stat.title} ${stat.artist}`, 5);
        const match =
          hits.find((s) => s.id === stat.songId) ??
          hits.find((s) => norm(s.title) === norm(stat.title) && norm(s.artist) === norm(stat.artist));
        if (match && !seen.has(match.id)) {
          seen.add(match.id);
          results.push(match);
        }
      } catch { /* skip */ }
    }
    return results;
  }

  async function load() {
    const ver = ++loadVersion;
    loading = true;
    error = '';
    try {
      const profile = await getListeningProfile();
      if (ver !== loadVersion) return;
      songs = await resolveTopSongs(profile.topSongs, 20);
    } catch (err) {
      if (ver !== loadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load.';
    } finally {
      if (ver === loadVersion) loading = false;
    }
  }

  function playAll() {
    if (!songs.length) return;
    const first = songs[0];
    focusTrack.set({ title: first.title, artist: first.artist, imageUrl: first.coverArtUrl, source: 'library', album: first.album });
    playingFrom.set({ type: 'playlist', name: 'Your Replay', href: HREF });
    playQueue(songs, 0);
  }

  function playSong(index: number) {
    const song = songs[index];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playingFrom.set({ type: 'playlist', name: 'Your Replay', href: HREF });
    playQueue(songs, index);
  }

  async function save() {
    if (!songs.length || saving) return;
    saving = true;
    try {
      await createPlaylist('Your Replay', songs.map((s) => s.id));
      requestLibraryRefresh();
      toast.success('Saved "Your Replay" to your library');
    } catch {
      toast.error('Failed to save playlist');
    } finally {
      saving = false;
    }
  }

  $effect(() => { load(); });
</script>

<div class="page-hero app-glass mb-6 flex gap-4 rounded-[2rem] p-5">
  <div class="relative h-36 w-36 shrink-0 overflow-hidden rounded-2xl shadow-lg">
    {#if songs.length >= 4}
      <div class="grid h-full w-full grid-cols-2 grid-rows-2">
        {#each songs.slice(0, 4) as song (song.id)}
          {#if song.coverArtUrl}
            <img class="h-full w-full object-cover" src={song.coverArtUrl} alt={song.title} loading="lazy" />
          {:else}
            <div class="h-full w-full bg-gradient-to-br from-slate-600 to-slate-800"></div>
          {/if}
        {/each}
      </div>
      <div class="absolute inset-0 bg-gradient-to-br from-black/10 via-transparent to-black/45"></div>
    {:else if loading}
      <div class="h-full w-full animate-pulse bg-muted"></div>
    {:else}
      <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-violet-600 to-indigo-900">
        <span class="text-4xl">🎵</span>
      </div>
    {/if}
  </div>

  <div class="flex flex-col justify-end gap-2">
    <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Playlist</p>
    <h2 class="app-section-title text-3xl font-bold tracking-tight">Your Replay</h2>
    {#if !loading && songs.length > 0}
      <p class="text-sm text-muted-foreground">{songs.length} songs · {totalDuration()}</p>
    {/if}
    <div class="mt-1 flex items-center gap-3">
      <button
        onclick={() => active ? togglePlayRequest.update((n) => n + 1) : playAll()}
        disabled={loading || !songs.length}
        class="flex size-14 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {#if active && $isPlaying}
          <Pause class="size-6 text-muted-foreground" fill="currentColor" />
        {:else}
          <Play class="size-6 translate-x-0.5 text-muted-foreground" fill="currentColor" />
        {/if}
      </button>

      <button
        onclick={save}
        disabled={saving || loading || !songs.length}
        class="app-round-button flex items-center gap-2 rounded-full px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-40"
      >
        {#if saving}
          <span class="block size-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
        {:else}
          <Save class="size-3.5" />
        {/if}
        Save to library
      </button>

      <button
        onclick={() => load()}
        disabled={loading}
        class="app-round-button grid size-10 place-items-center rounded-full transition disabled:opacity-40"
        aria-label="Refresh"
      >
        <RefreshCw class="size-4 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>
</div>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}

{#if loading}
  <p class="mb-3 text-sm text-muted-foreground">Loading your top songs…</p>
{/if}

{#if songs.length}
  <div class="page-section mt-2">
    <div
      class="grid items-center gap-4 border-b border-border/40 px-4 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
      style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
    >
      <span class="text-center">#</span>
      <span>Title</span>
      <span class="hidden md:block">Album</span>
      <span class="text-right">Duration</span>
    </div>

    <div class="mt-1 space-y-0.5">
      {#each songs as song, index (song.id)}
        {@const isCurrentTrack = song.id === currentTrackId && active}
        <SongContextMenu {song} onplay={() => playSong(index)}>
          <button
            class="stagger-row group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5 {isCurrentTrack ? 'bg-primary/5' : ''}"
            style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
            style:--stagger-index={index}
            onclick={() => isCurrentTrack ? togglePlayRequest.update((n) => n + 1) : playSong(index)}
          >
            <span class="relative mx-auto flex h-7 w-7 shrink-0 items-center justify-center">
              {#if isCurrentTrack}
                <span class="flex items-end gap-[2px] transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">
                  <span class="w-[3px] origin-bottom rounded-[1px] bg-primary" style="height:12px;animation:equalizer 0.8s ease-in-out infinite 0s;animation-play-state:{$isPlaying?'running':'paused'}"></span>
                  <span class="w-[3px] origin-bottom rounded-[1px] bg-primary" style="height:8px;animation:equalizer 0.8s ease-in-out infinite 0.25s;animation-play-state:{$isPlaying?'running':'paused'}"></span>
                  <span class="w-[3px] origin-bottom rounded-[1px] bg-primary" style="height:12px;animation:equalizer 0.8s ease-in-out infinite 0.5s;animation-play-state:{$isPlaying?'running':'paused'}"></span>
                </span>
                <span class="absolute inset-0 flex scale-50 items-center justify-center text-primary opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
                  {#if $isPlaying}<Pause class="size-4" fill="currentColor" />{:else}<Play class="size-4" fill="currentColor" />{/if}
                </span>
              {:else}
                <span class="absolute inset-0 flex items-center justify-center text-sm tabular-nums text-muted-foreground transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">{index + 1}</span>
                <span class="absolute inset-0 flex scale-50 items-center justify-center opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
                  <Play class="size-4" fill="currentColor" />
                </span>
              {/if}
            </span>

            <div class="flex min-w-0 items-center gap-3">
              {#if song.coverArtUrl}
                <img class="size-10 shrink-0 rounded-md object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
              {:else}
                <div class="grid size-10 shrink-0 place-items-center rounded-md bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold shadow-md">
                  {song.title.slice(0, 2).toUpperCase()}
                </div>
              {/if}
              <div class="min-w-0">
                <p class="whitespace-normal break-words text-sm font-medium leading-tight transition-colors duration-150 group-hover:text-foreground {isCurrentTrack ? 'text-primary' : ''}">{song.title}</p>
                <SongArtistLinks artist={song.artist} class="truncate text-xs text-muted-foreground" linkClass="hover:underline hover:text-foreground transition-colors duration-150" />
              </div>
            </div>

            <span
              role="link" tabindex="0"
              class="hidden truncate text-sm text-muted-foreground transition-colors duration-150 hover:text-foreground hover:underline md:block cursor-pointer"
              onclick={(e) => { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); }}
              onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); } }}
            >{song.album}</span>

            <span class="text-right text-xs tabular-nums text-muted-foreground">{formatClockDuration(song.duration ?? 0)}</span>
          </button>
        </SongContextMenu>
      {/each}
    </div>
  </div>
{:else if !loading && !error}
  <p class="text-sm text-muted-foreground">Start listening to music to build your replay.</p>
{/if}
