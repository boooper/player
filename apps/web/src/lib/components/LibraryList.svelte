<script lang="ts">
  import { Heart, ListMusic, Play, Pause, Music2, Disc3, Mic2 } from '@lucide/svelte';
  import { page } from '$app/state';
  import { isPlaying, subsonicPlaylists, starredSongIds } from '$lib/stores/player';
  import type { Song } from '$lib/api';

  let {
    likedArtists,
    artistPhotos,
    starredSongs,
    selectedPlaylistId,
    onPlayPlaylist,
  }: {
    likedArtists: string[];
    artistPhotos: Record<string, string>;
    starredSongs: Song[];
    selectedPlaylistId: string;
    onPlayPlaylist: (id: string) => void;
  } = $props();

  const starredSongCount = $derived($starredSongIds.size);

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }
</script>

<ul class="m-0 list-none px-2 pb-4 pt-1 space-y-0.5">

  <!-- Browse nav links -->
  {#each [
    { href: '/songs',   label: 'Songs',   Icon: Music2 },
    { href: '/albums',  label: 'Albums',  Icon: Disc3 },
    { href: '/artists', label: 'Artists', Icon: Mic2 },
  ] as nav (nav.href)}
    <li>
      <a
        href={nav.href}
        class="flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10 {page.url.pathname === nav.href ? 'bg-white/10 text-foreground' : 'text-muted-foreground'}"
      >
        <div class="flex size-10 shrink-0 items-center justify-center rounded-md bg-secondary">
          <nav.Icon class="size-4" />
        </div>
        <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
          <p class="truncate text-sm font-medium leading-tight text-foreground">{nav.label}</p>
          <p class="mt-0.5 text-xs text-muted-foreground">Browse library</p>
        </div>
      </a>
    </li>
  {/each}

  <li class="px-2 py-1 group-data-[collapsible=icon]:hidden"><div class="h-px bg-white/5"></div></li>

  <!-- Liked Songs -->
  <li>
    <a
      href="/favorites"
      class="flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10"
    >
      <div class="relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
        {#if starredSongs.length >= 4}
          <div class="grid h-full w-full grid-cols-2 grid-rows-2">
            {#each Array(4) as _, i}
              <img src={starredSongs[i].coverArtUrl} alt="" class="h-full w-full object-cover" />
            {/each}
          </div>
        {:else}
          <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-indigo-500 to-indigo-800">
            <Heart class="size-5 text-white" fill="white" />
          </div>
        {/if}
      </div>
      <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
        <p class="truncate text-sm font-medium leading-tight text-foreground">Liked Songs</p>
        <p class="mt-0.5 truncate text-xs text-muted-foreground">Playlist &bull; {starredSongCount} songs</p>
      </div>
    </a>
  </li>

  <!-- Playlists -->
  {#each $subsonicPlaylists as playlist (playlist.id)}
    {@const isActive = selectedPlaylistId === playlist.id}
    <li>
      <a
        href={`/playlist/${encodeURIComponent(playlist.id)}`}
        class="group/row flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10 {isActive ? 'bg-white/5' : ''}"
      >
        <div class="group/cover relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
          {#if playlist.coverArtUrl}
            <img src={playlist.coverArtUrl} alt={playlist.name} class="h-full w-full object-cover" />
          {:else}
            <div class="flex h-full w-full items-center justify-center bg-secondary">
              <ListMusic class="size-4 text-muted-foreground" />
            </div>
          {/if}
          <button
            class="absolute inset-0 flex items-center justify-center rounded-md bg-black/60 opacity-0 transition-opacity group-hover/row:opacity-100 {isActive && $isPlaying ? '!opacity-100' : ''}"
            onclick={(e) => { e.preventDefault(); e.stopPropagation(); onPlayPlaylist(playlist.id); }}
          >
            {#if isActive && $isPlaying}
              <Pause class="size-4 text-white" />
            {:else}
              <Play class="size-4 text-white" />
            {/if}
          </button>
          {#if isActive && $isPlaying}
            <div class="absolute inset-0 flex items-end justify-center gap-[2px] rounded-md bg-black/50 pb-1.5 pointer-events-none">
              <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
              <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
              <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
            </div>
          {/if}
        </div>
        <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
          <p class="truncate text-sm font-medium leading-tight {isActive ? 'text-primary' : 'text-foreground'}">{playlist.name}</p>
          <p class="mt-0.5 truncate text-xs text-muted-foreground">Playlist &bull; {playlist.songCount} songs</p>
        </div>
      </a>
    </li>
  {/each}

  {#if likedArtists.length > 0}
    <li class="px-2 py-1 group-data-[collapsible=icon]:hidden"><div class="h-px bg-white/5"></div></li>
  {/if}

  <!-- Liked Artists -->
  {#each likedArtists as artist (artist)}
    <li>
      <a
        href={`/artist/${encodeURIComponent(artist)}`}
        class="flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10"
      >
        <div class="size-10 shrink-0 flex-none overflow-hidden rounded-full">
          {#if artistPhotos[artist]}
            <img src={artistPhotos[artist]} alt={artist} class="h-full w-full object-cover" />
          {:else}
            <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-slate-600 to-slate-800 text-xs font-bold text-white/70">{initials(artist)}</div>
          {/if}
        </div>
        <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
          <p class="truncate text-sm font-medium leading-tight text-foreground">{artist}</p>
          <p class="mt-0.5 text-xs text-muted-foreground">Artist</p>
        </div>
      </a>
    </li>
  {/each}

</ul>
