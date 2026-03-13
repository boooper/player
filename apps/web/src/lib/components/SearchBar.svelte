<script lang="ts">
  import { goto } from '$app/navigation';
  import { Search, Clock, X, Music, User } from '@lucide/svelte';
  import { Input } from '$lib/components/ui';
  import { searchSongs, type Song } from '$lib/api';
  import { fetchAudioDbArtistPhoto } from '$lib/audiodb';

  let {
    onPlaySong,
    onGotoArtist,
  }: {
    onPlaySong: (song: Song) => void;
    onGotoArtist: (artist: string) => void;
  } = $props();

  let value = $state('');
  let dropdownSongs = $state<Song[]>([]);
  let dropdownArtists = $state<string[]>([]);
  let artistPhotos = $state<Record<string, string>>({});
  let dropdownOpen = $state(false);
  let searchFocused = $state(false);
  let dropdownLoading = $state(false);
  let recentSearches = $state<string[]>([]);

  const RECENT_KEY = 'naviarr_recent_searches';

  export function setQuery(q: string) {
    value = q;
  }

  export function reset() {
    dropdownOpen = false;
  }

  function loadRecentSearches() {
    try {
      recentSearches = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    } catch {
      recentSearches = [];
    }
  }

  function saveRecentSearch(q: string) {
    const trimmed = q.trim();
    if (!trimmed) return;
    const updated = [trimmed, ...recentSearches.filter((r) => r.toLowerCase() !== trimmed.toLowerCase())].slice(0, 8);
    recentSearches = updated;
    localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
  }

  function removeRecentSearch(q: string) {
    const updated = recentSearches.filter((r) => r !== q);
    recentSearches = updated;
    localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
  }

  let dropdownTimer = 0;

  function onInput() {
    const q = value.trim();
    clearTimeout(dropdownTimer);
    if (q.length < 2) {
      dropdownSongs = [];
      dropdownArtists = [];
      dropdownOpen = true;
      return;
    }
    dropdownLoading = true;
    dropdownOpen = true;
    dropdownTimer = window.setTimeout(async () => {
      try {
        const songs = await searchSongs(q, 8);
        dropdownSongs = songs.slice(0, 5);
        const seen = new Set<string>();
        dropdownArtists = songs
          .map((s) => s.artist)
          .filter((a) => { if (seen.has(a)) return false; seen.add(a); return true; })
          .slice(0, 4);
        fetchPhotos(dropdownArtists);
      } catch {
        dropdownSongs = [];
        dropdownArtists = [];
      } finally {
        dropdownLoading = false;
      }
    }, 280);
  }

  async function fetchPhotos(names: string[]) {
    const missing = names.filter((n) => artistPhotos[n] === undefined);
    if (!missing.length) return;
    const patch: Record<string, string> = {};
    for (const n of missing) patch[n] = '';
    artistPhotos = { ...artistPhotos, ...patch };
    await Promise.all(
      missing.map(async (name) => {
        const url = await fetchAudioDbArtistPhoto(name);
        artistPhotos = { ...artistPhotos, [name]: url };
      })
    );
  }

  function onSubmit(event: SubmitEvent) {
    event.preventDefault();
    dropdownOpen = false;
    const query = value.trim();
    if (!query) return;
    saveRecentSearch(query);
    goto(`/search?q=${encodeURIComponent(query)}`);
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  loadRecentSearches();
</script>

<div class="relative transition-all duration-300 ease-in-out {searchFocused ? 'w-full max-w-2xl' : 'w-full max-w-lg'}">
  <form onsubmit={onSubmit}>
    <Search class="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 transition-colors duration-200 {searchFocused ? 'text-primary' : 'text-muted-foreground'}" />
    <Input
      bind:value
      class="h-10 rounded-full border-transparent bg-white/[0.06] pl-11 pr-4 text-sm placeholder:text-muted-foreground/60 transition-all duration-300 focus-visible:border-primary/40 focus-visible:bg-white/[0.09] focus-visible:ring-0 focus-visible:shadow-[0_0_0_3px_oklch(0.645_0.246_16.439_/_0.15)]"
      placeholder="What do you want to play?"
      oninput={onInput}
      onfocus={() => {
        searchFocused = true;
        dropdownOpen = true;
        if (value.trim().length >= 2) onInput();
      }}
      onblur={() => { searchFocused = false; dropdownOpen = false; }}
    />
  </form>

  <!-- Recent searches dropdown -->
  {#if dropdownOpen && value.trim().length < 2 && recentSearches.length > 0}
    <div
      role="listbox"
      aria-label="Recent searches"
      tabindex="-1"
      class="absolute left-0 top-full z-50 mt-1 w-full max-w-lg overflow-hidden rounded-xl border border-border bg-popover shadow-2xl"
      onmousedown={(e) => e.preventDefault()}
    >
      <div class="px-3 pb-2 pt-2">
        <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Recent searches</p>
        <div class="space-y-0.5">
          {#each recentSearches as term (term)}
            <div class="flex items-center gap-1">
              <button
                class="flex flex-1 items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                onclick={() => { value = term; dropdownOpen = false; saveRecentSearch(term); goto(`/search?q=${encodeURIComponent(term)}`); }}
              >
                <Clock class="size-3.5 shrink-0 text-muted-foreground" />
                <span class="truncate text-sm">{term}</span>
              </button>
              <button
                class="rounded p-1 text-muted-foreground hover:text-foreground"
                aria-label="Remove"
                onclick={() => removeRecentSearch(term)}
              >
                <X class="size-3.5" />
              </button>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- Results dropdown -->
  {#if dropdownOpen && value.trim().length >= 2}
    <div
      role="listbox"
      aria-label="Search results"
      tabindex="-1"
      class="absolute left-0 top-full z-50 mt-1 w-full max-w-lg overflow-hidden rounded-xl border border-border bg-popover shadow-2xl"
      onmousedown={(e) => e.preventDefault()}
    >
      {#if dropdownLoading}
        <div class="px-4 py-3 text-sm text-muted-foreground">Searching…</div>
      {:else}
        {#if dropdownSongs.length > 0}
          <div class="px-3 pb-1 pt-2">
            <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Songs</p>
            <div class="space-y-0.5">
              {#each dropdownSongs as song (song.id)}
                <button
                  class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                  onclick={() => { onPlaySong(song); dropdownOpen = false; }}
                >
                  {#if song.coverArtUrl}
                    <img class="size-8 rounded object-cover" src={song.coverArtUrl} alt={song.title} />
                  {:else}
                    <span class="grid size-8 place-items-center rounded bg-secondary text-[10px] font-bold">{initials(song.title)}</span>
                  {/if}
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-sm font-medium">{song.title}</span>
                    <span class="block truncate text-xs text-muted-foreground">{song.artist}</span>
                  </span>
                  <Music class="ml-auto size-3.5 shrink-0 text-muted-foreground" />
                </button>
              {/each}
            </div>
          </div>
        {/if}
        {#if dropdownArtists.length > 0}
          {#if dropdownSongs.length > 0}
            <div class="mx-3 my-1 border-t border-border/60"></div>
          {/if}
          <div class="px-3 pb-2 pt-1">
            <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Artists</p>
            <div class="space-y-0.5">
              {#each dropdownArtists as artist (artist)}
                <button
                  class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                  onclick={() => { onGotoArtist(artist); dropdownOpen = false; }}
                >
                  {#if artistPhotos[artist]}
                    <img src={artistPhotos[artist]} alt={artist} class="size-8 shrink-0 rounded-full object-cover" />
                  {:else}
                    <span class="grid size-8 shrink-0 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-[10px] font-bold">{initials(artist)}</span>
                  {/if}
                  <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist}</span>
                  <User class="ml-auto size-3.5 shrink-0 text-muted-foreground" />
                </button>
              {/each}
            </div>
          </div>
        {/if}
        {#if dropdownSongs.length === 0 && dropdownArtists.length === 0}
          <div class="px-4 py-3 text-sm text-muted-foreground">No results found.</div>
        {/if}
      {/if}
      <div class="border-t border-border/60 px-3 py-1.5">
        <button
          class="text-xs text-muted-foreground hover:text-foreground"
          onclick={() => { dropdownOpen = false; goto(`/search?q=${encodeURIComponent(value.trim())}`); }}
        >See all results for "{value}"</button>
      </div>
    </div>
  {/if}
</div>
