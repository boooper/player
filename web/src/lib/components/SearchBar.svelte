<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Search, Clock, X, Music, User, ArrowRight } from '@lucide/svelte';
  import { Input } from '$lib/components/ui';
  import { searchSongs, type Song } from '$lib/servers';
  import { getArtistArtworkMap } from '$lib/discovery';
  import { readUiJson, writeUiJson } from '$lib/ui-storage';
  import { initials, uniqueBy } from '$lib/utils';

  let {
    onPlaySong,
    onGotoArtist,
    onclose,
  }: {
    onPlaySong: (song: Song) => void;
    onGotoArtist: (artist: string) => void;
    onclose?: () => void;
  } = $props();

  let inputEl = $state<HTMLInputElement | null>(null);

  export function focus() {
    inputEl?.focus();
  }

  let value = $state('');
  let dropdownSongs = $state<Song[]>([]);
  let dropdownArtists = $state<string[]>([]);
  let artistPhotos = $state<Record<string, string>>({});
  let dropdownOpen = $state(false);
  let searchFocused = $state(false);
  let dropdownLoading = $state(false);
  let recentSearches = $state<string[]>([]);
  const RECENT_KEY = 'madrify_recent_searches';
  const LEGACY_RECENT_KEY = 'naviarr_recent_searches';

  const hasQuery = $derived(value.trim().length >= 2);

  export function setQuery(q: string) {
    value = q;
  }

  export function reset() {
    dropdownOpen = false;
  }

  async function loadRecentSearches() {
    recentSearches = await readUiJson<string[]>(RECENT_KEY, [], [LEGACY_RECENT_KEY]);
  }

  async function openSearchDropdown() {
    searchFocused = true;
    if (!recentSearches.length) {
      await loadRecentSearches();
    }
    dropdownOpen = true;
    if (value.trim().length >= 2) onInput();
  }

  function saveRecentSearch(q: string) {
    const trimmed = q.trim();
    if (!trimmed) return;
    const updated = [trimmed, ...recentSearches.filter((r) => r.toLowerCase() !== trimmed.toLowerCase())].slice(0, 8);
    recentSearches = updated;
    void writeUiJson(RECENT_KEY, updated, [LEGACY_RECENT_KEY]);
  }

  function removeRecentSearch(q: string) {
    const updated = recentSearches.filter((r) => r !== q);
    recentSearches = updated;
    void writeUiJson(RECENT_KEY, updated, [LEGACY_RECENT_KEY]);
  }

  function clearAllRecents() {
    recentSearches = [];
    void writeUiJson(RECENT_KEY, [], [LEGACY_RECENT_KEY]);
  }

  let dropdownTimer = 0;
  let blurTimer = 0;

  function cancelClose() {
    clearTimeout(blurTimer);
  }

  function scheduleClose() {
    cancelClose();
    blurTimer = window.setTimeout(() => {
      searchFocused = false;
      dropdownOpen = false;
      onclose?.();
    }, 140);
  }

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
        dropdownArtists = uniqueBy(songs, (s) => s.artist).map((s) => s.artist).slice(0, 4);
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
    const next = await getArtistArtworkMap(missing);
    artistPhotos = { ...artistPhotos, ...next };
  }

  function onSubmit(event: SubmitEvent) {
    event.preventDefault();
    dropdownOpen = false;
    const query = value.trim();
    if (!query) return;
    saveRecentSearch(query);
    value = '';
    goto(`/search?q=${encodeURIComponent(query)}`);
  }


  onMount(() => {
    void loadRecentSearches();
  });
</script>

<div
  class="search-shell relative w-full max-w-xs"
  class:is-active={searchFocused || dropdownOpen}
  class:is-idle={!searchFocused && !dropdownOpen}
>
  <form onsubmit={onSubmit}>
    <div class="search-backdrop absolute inset-0 rounded-full" aria-hidden="true"></div>
    <span class="search-icon-wrap pointer-events-none absolute left-3 top-1/2 -translate-y-1/2">
      <Search class="size-3.5 {searchFocused ? 'text-primary/90' : 'text-muted-foreground'}" />
    </span>
    <div class="search-input-wrap">
      <Input
        bind:ref={inputEl}
        bind:value
        class="relative h-8 rounded-full border-white/8 bg-transparent pl-9 pr-4 text-xs text-foreground placeholder:text-muted-foreground/55 shadow-none backdrop-blur-0 focus-visible:border-primary/35 focus-visible:bg-transparent focus-visible:ring-0 focus-visible:shadow-none"
        placeholder="What do you want to play?"
        oninput={onInput}
        onfocus={() => { cancelClose(); void openSearchDropdown(); }}
        onblur={scheduleClose}
      />
    </div>
  </form>

  {#if dropdownOpen}
    <div
      role="listbox"
      aria-label={hasQuery ? 'Search results' : 'Recent searches'}
      tabindex="-1"
      class="search-dropdown pointer-events-auto absolute left-0 top-full z-[120] mt-3 w-full overflow-hidden rounded-[20px]"
      onpointerdown={cancelClose}
    >
      {#if !hasQuery}
        <!-- Recents panel -->
        {#if recentSearches.length > 0}
          <div class="px-2 pt-3 pb-2">
            <div class="mb-2 flex items-center justify-between px-2">
              <span class="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/60">Recent</span>
              <button
                type="button"
                class="text-[11px] text-muted-foreground/40 transition-colors hover:text-muted-foreground"
                onclick={clearAllRecents}
              >
                Clear all
              </button>
            </div>
            {#each recentSearches as term, i (term)}
              <div
                class="search-row group flex items-center gap-1 rounded-xl"
                style="--stagger-index: {i}"
              >
                <button
                  type="button"
                  class="flex flex-1 items-center gap-3 rounded-[16px] px-2.5 py-2.5 text-left transition-colors hover:bg-white/[0.04]"
                  onclick={() => { value = term; dropdownOpen = false; saveRecentSearch(term); goto(`/search?q=${encodeURIComponent(term)}`); }}
                >
                  <span class="grid size-7 shrink-0 place-items-center rounded-lg bg-transparent">
                    <Clock class="size-3.5 text-muted-foreground/70" />
                  </span>
                  <span class="flex-1 truncate text-sm">{term}</span>
                  <ArrowRight class="size-3.5 shrink-0 text-muted-foreground/0 transition-opacity group-hover:text-muted-foreground/50" />
                </button>
                <button
                  type="button"
                  class="rounded-lg p-1.5 text-muted-foreground/0 transition-colors group-hover:text-muted-foreground/40 hover:!text-foreground"
                  aria-label="Remove"
                  onclick={() => removeRecentSearch(term)}
                >
                  <X class="size-3.5" />
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <!-- Empty state -->
            <div class="flex flex-col items-center gap-2 px-4 py-8 text-center">
            <span class="grid size-11 place-items-center rounded-2xl bg-transparent">
              <Search class="size-5 text-muted-foreground/40" />
            </span>
            <div>
              <p class="text-sm font-medium text-muted-foreground/70">Search your library</p>
              <p class="mt-0.5 text-xs text-muted-foreground/40">Songs, artists, albums…</p>
            </div>
          </div>
        {/if}

      {:else}
        <!-- Results panel -->
        {#if dropdownLoading}
          <div class="flex items-center gap-2 px-5 py-4">
            <div class="loading-dot"></div>
            <div class="loading-dot" style="animation-delay: 140ms"></div>
            <div class="loading-dot" style="animation-delay: 280ms"></div>
            <span class="ml-1 text-sm text-muted-foreground/60">Searching…</span>
          </div>
        {:else}
          {#if dropdownSongs.length > 0}
            <div class="px-2 pt-3 pb-1">
              <p class="mb-2 px-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/60">Songs</p>
              {#each dropdownSongs as song, i (song.id)}
                <button
                  type="button"
                  class="search-row flex w-full items-center gap-3 rounded-[16px] px-2.5 py-2.5 text-left transition-colors hover:bg-white/[0.04]"
                  style="--stagger-index: {i}"
                  onclick={() => { onPlaySong(song); dropdownOpen = false; }}
                >
                  {#if song.coverArtUrl}
                    <img class="size-9 shrink-0 rounded-lg object-cover" src={song.coverArtUrl} alt={song.title} />
                  {:else}
                    <span class="grid size-9 shrink-0 place-items-center rounded-lg bg-transparent text-[10px] font-bold text-muted-foreground">{initials(song.title)}</span>
                  {/if}
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-sm font-medium leading-tight">{song.title}</span>
                    <span class="block truncate text-xs text-muted-foreground/70">{song.artist}</span>
                  </span>
                  <Music class="ml-auto size-3.5 shrink-0 text-muted-foreground/30" />
                </button>
              {/each}
            </div>
          {/if}

          {#if dropdownArtists.length > 0}
            {#if dropdownSongs.length > 0}
              <div class="mx-3 my-1 border-t border-white/[0.06]"></div>
            {/if}
            <div class="px-2 pb-2 pt-1">
              <p class="mb-2 px-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/60">Artists</p>
              {#each dropdownArtists as artist, i (artist)}
                <button
                  type="button"
                  class="search-row flex w-full items-center gap-3 rounded-[16px] px-2.5 py-2.5 text-left transition-colors hover:bg-white/[0.04]"
                  style="--stagger-index: {i + (dropdownSongs.length)}"
                  onclick={() => { onGotoArtist(artist); dropdownOpen = false; }}
                >
                  {#if artistPhotos[artist]}
                    <img src={artistPhotos[artist]} alt={artist} class="size-9 shrink-0 rounded-full object-cover" />
                  {:else}
                    <span class="grid size-9 shrink-0 place-items-center rounded-full bg-gradient-to-br from-primary/30 to-primary/10 text-[11px] font-bold text-primary/80">{initials(artist)}</span>
                  {/if}
                  <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist}</span>
                  <User class="ml-auto size-3.5 shrink-0 text-muted-foreground/30" />
                </button>
              {/each}
            </div>
          {/if}

          {#if dropdownSongs.length === 0 && dropdownArtists.length === 0}
            <div class="px-4 py-6 text-center">
              <p class="text-sm text-muted-foreground/60">No results for <span class="text-foreground/80">"{value}"</span></p>
              <p class="mt-1 text-xs text-muted-foreground/40">Try a different spelling or keyword</p>
            </div>
          {/if}

          <div class="border-t border-white/[0.06] px-2 py-1.5">
            <button
              type="button"
              class="flex w-full items-center justify-between rounded-[16px] px-3 py-2.5 text-xs text-muted-foreground/50 transition-colors hover:bg-white/[0.04] hover:text-muted-foreground"
              onclick={() => { dropdownOpen = false; saveRecentSearch(value.trim()); goto(`/search?q=${encodeURIComponent(value.trim())}`); }}
            >
              <span>See all results for <span class="font-medium text-foreground/70">"{value}"</span></span>
              <ArrowRight class="size-3.5" />
            </button>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .search-shell {
    transition: transform 220ms ease, filter 220ms ease;
  }

  .search-shell.is-idle {
    transform: translateY(0) scale(1);
  }

  .search-shell.is-active {
    transform: translateY(0) scale(1.006);
  }

  .search-backdrop {
    background: rgb(20 20 26 / 0.96);
    border: 1px solid rgb(255 255 255 / 0.07);
    box-shadow: none;
    opacity: 1;
    transition:
      background-color 180ms ease,
      border-color 180ms ease;
  }

  .search-icon-wrap {
    transition: color 180ms ease, opacity 180ms ease;
    opacity: 0.82;
  }

  .search-input-wrap :global(input) {
    transition:
      background-color 220ms ease,
      border-color 220ms ease,
      color 220ms ease;
  }

  .search-dropdown {
    border: 1px solid rgb(255 255 255 / 0.07);
    background: rgb(20 20 26 / 0.98);
    box-shadow: 0 16px 36px rgb(0 0 0 / 0.34);
    transform-origin: top center;
    animation: search-dropdown-in 200ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
  }

  /* Staggered row entrance — mirrors .stagger-row in app.css */
  .search-row {
    animation: search-row-in 220ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
    animation-delay: min(calc(var(--stagger-index, 0) * 28ms + 60ms), 280ms);
  }

  .search-shell.is-active .search-backdrop {
    border-color: rgb(255 255 255 / 0.1);
    background: rgb(24 24 32 / 0.98);
    border-radius: 9999px;
  }

  .search-shell.is-active .search-icon-wrap {
    opacity: 1;
  }

  .search-shell :global(input::placeholder) {
    transition: color 180ms ease, opacity 180ms ease, letter-spacing 180ms ease;
  }

  .search-shell.is-active :global(input::placeholder),
  .search-shell :global(input:hover::placeholder) {
    color: rgb(255 255 255 / 0.5);
    letter-spacing: 0.01em;
  }

  .loading-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: hsl(var(--primary) / 0.5);
    animation: loading-pulse 900ms ease-in-out infinite;
  }

  @keyframes loading-pulse {
    0%, 100% { opacity: 0.25; transform: scale(0.8); }
    50%       { opacity: 1;    transform: scale(1); }
  }

  @keyframes search-dropdown-in {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes search-row-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
