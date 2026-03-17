<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { goto } from '$app/navigation';
  import { Search, Clock, X, Music, User } from '@lucide/svelte';
  import { Input } from '$lib/components/ui';
  import { searchSongs, type Song } from '$lib/servers';
  import { getArtistArtworkMap } from '$lib/discovery';
  import { readUiJson, writeUiJson } from '$lib/ui-storage';

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
  const RECENT_KEY = 'madrify_recent_searches';
  const LEGACY_RECENT_KEY = 'naviarr_recent_searches';

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
        const seen = new SvelteSet<string>();
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
    const next = await getArtistArtworkMap(missing);
    artistPhotos = { ...artistPhotos, ...next };
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

  onMount(() => {
    void loadRecentSearches();
  });
</script>

<div
  class="search-shell relative w-full max-w-xl"
  class:is-active={searchFocused || dropdownOpen}
  class:is-idle={!searchFocused && !dropdownOpen}
>
  <form onsubmit={onSubmit}>
    <div class="search-backdrop absolute inset-0 rounded-full" aria-hidden="true"></div>
    <div class="search-highlight absolute inset-x-10 top-0 h-px" aria-hidden="true"></div>
    <span class="search-icon-wrap pointer-events-none absolute left-4 top-1/2 -translate-y-1/2">
      <Search class="size-4 {searchFocused ? 'text-primary' : 'text-muted-foreground'}" />
    </span>
    <div class="search-input-wrap">
      <Input
      bind:value
      class="relative h-11 rounded-full border-white/10 bg-white/[0.06] pl-11 pr-4 text-sm text-foreground placeholder:text-muted-foreground/60 shadow-[inset_0_1px_0_rgb(255_255_255_/_0.05)] backdrop-blur-xl focus-visible:border-primary/35 focus-visible:bg-white/[0.08] focus-visible:ring-0 focus-visible:shadow-none"
      placeholder="What do you want to play?"
      oninput={onInput}
      onfocus={() => {
        void openSearchDropdown();
      }}
      onblur={() => { searchFocused = false; dropdownOpen = false; }}
    />
    </div>
  </form>

  <!-- Recent searches dropdown -->
  {#if dropdownOpen && value.trim().length < 2 && recentSearches.length > 0}
    <div
      role="listbox"
      aria-label="Recent searches"
      tabindex="-1"
      class="search-dropdown app-glass absolute left-0 top-full z-50 mt-2 w-full max-w-lg overflow-hidden rounded-2xl"
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
      class="search-dropdown app-glass absolute left-0 top-full z-50 mt-2 w-full max-w-lg overflow-hidden rounded-2xl"
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
                    <span class="block whitespace-normal break-words text-sm font-medium leading-tight">{song.title}</span>
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

<style>
  .search-shell {
    transition: transform 220ms ease, filter 220ms ease;
  }

  .search-shell.is-idle {
    transform: translateY(0) scale(1);
  }

  .search-shell.is-active {
    transform: translateY(1px) scale(1.012);
  }

  .search-backdrop {
    background:
      radial-gradient(circle at 14% 50%, hsl(var(--primary) / 0.2), transparent 18%),
      linear-gradient(180deg, rgb(255 255 255 / 0.05), rgb(255 255 255 / 0.015)),
      rgb(20 20 24 / 0.52);
    border: 1px solid rgb(255 255 255 / 0.08);
    box-shadow:
      0 14px 32px rgb(0 0 0 / 0.18),
      inset 0 1px 0 rgb(255 255 255 / 0.06);
    opacity: 0.92;
    transition:
      opacity 220ms ease,
      transform 220ms ease,
      box-shadow 220ms ease,
      border-color 220ms ease;
  }

  .search-highlight {
    opacity: 0.4;
    background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.32), transparent);
    transition: opacity 220ms ease, transform 220ms ease;
  }

  .search-icon-wrap {
    transition: color 180ms ease, transform 220ms ease, opacity 180ms ease;
    opacity: 0.82;
  }

  .search-input-wrap :global(input) {
    transition:
      background-color 220ms ease,
      border-color 220ms ease,
      color 220ms ease;
  }

  .search-dropdown {
    transform-origin: top center;
    animation: search-dropdown-in 190ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
  }

  .search-shell.is-active .search-backdrop {
    opacity: 1;
    border-color: rgb(255 255 255 / 0.11);
    box-shadow:
      0 18px 40px rgb(0 0 0 / 0.22),
      0 0 0 1px hsl(var(--primary) / 0.12),
      inset 0 1px 0 rgb(255 255 255 / 0.08);
  }

  .search-shell.is-active .search-highlight {
    opacity: 0.72;
    transform: scaleX(1.04);
  }

  .search-shell.is-active .search-icon-wrap {
    transform: translateY(-50%) scale(1.06);
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

  @keyframes search-dropdown-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }

    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
