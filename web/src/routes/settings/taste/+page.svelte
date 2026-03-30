<script lang="ts">
  import { onMount } from 'svelte';
  import { Search, X, Loader2, Plus, Music2 } from '@lucide/svelte';
  import {
    fetchLikedArtists,
    saveLikedArtist,
    removeLikedArtist,
    type StoredLikedArtist,
  } from '$lib/servers';
  import { searchArtists } from '$lib/discovery';
  import type { UnifiedArtist } from '$lib/data/types';
  import { initials } from '$lib/utils';

  let likedArtists = $state<StoredLikedArtist[]>([]);
  let loadingLiked = $state(true);
  let query = $state('');
  let searchResults = $state<UnifiedArtist[]>([]);
  let searching = $state(false);
  let pending = $state(new Set<string>());

  const likedNames = $derived(new Set(likedArtists.map((a) => a.name.toLowerCase())));

  let searchTimer: ReturnType<typeof setTimeout>;

  function onQueryInput() {
    clearTimeout(searchTimer);
    const q = query.trim();
    if (!q) { searchResults = []; searching = false; return; }
    searching = true;
    searchTimer = setTimeout(async () => {
      try {
        searchResults = await searchArtists(q, 12);
      } catch {
        searchResults = [];
      } finally {
        searching = false;
      }
    }, 300);
  }

  async function addArtist(name: string) {
    const key = name.toLowerCase();
    pending = new Set([...pending, key]);
    try {
      const artist = await saveLikedArtist(name);
      likedArtists = [...likedArtists, artist];
    } finally {
      const next = new Set(pending); next.delete(key); pending = next;
    }
  }

  async function removeArtist(name: string) {
    const key = name.toLowerCase();
    pending = new Set([...pending, key]);
    try {
      await removeLikedArtist(name);
      likedArtists = likedArtists.filter((a) => a.name.toLowerCase() !== key);
    } finally {
      const next = new Set(pending); next.delete(key); pending = next;
    }
  }

  onMount(async () => {
    try { likedArtists = await fetchLikedArtists(); }
    finally { loadingLiked = false; }
  });
</script>

<div class="space-y-6">

  <!-- Search -->
  <section class="rounded-xl border border-border/70 bg-card">
    <div class="border-b border-border/60 px-5 py-4">
      <h2 class="font-semibold">Add Artists</h2>
      <p class="mt-0.5 text-xs text-muted-foreground">Search your library or any artist on Last.fm to build your taste profile.</p>
    </div>
    <div class="px-5 py-4">
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <input
          type="text"
          placeholder="Search artists…"
          class="w-full rounded-lg border border-border bg-secondary/40 py-2.5 pl-9 pr-9 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/40"
          bind:value={query}
          oninput={onQueryInput}
        />
        {#if searching}
          <Loader2 class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 animate-spin text-muted-foreground" />
        {:else if query}
          <button
            class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onclick={() => { query = ''; searchResults = []; }}
            aria-label="Clear search"
          ><X class="size-4" /></button>
        {/if}
      </div>

      {#if searchResults.length > 0}
        <ul class="mt-3 divide-y divide-border/40 overflow-hidden rounded-lg border border-border/60">
          {#each searchResults as artist (artist.id)}
            {@const isLiked = likedNames.has(artist.name.toLowerCase())}
            {@const isPending = pending.has(artist.name.toLowerCase())}
            <li class="flex items-center gap-3 bg-secondary/10 px-4 py-2.5 hover:bg-secondary/30 transition-colors">
              {#if artist.imageUrl}
                <img src={artist.imageUrl} alt={artist.name} class="size-8 shrink-0 rounded-full object-cover" />
              {:else}
                <div class="grid size-8 shrink-0 place-items-center rounded-full bg-secondary text-xs font-bold text-muted-foreground">
                  {initials(artist.name)}
                </div>
              {/if}
              <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist.name}</span>
              {#if isLiked}
                <button
                  class="flex shrink-0 items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
                  onclick={() => removeArtist(artist.name)}
                  disabled={isPending}
                >
                  {#if isPending}<Loader2 class="size-3 animate-spin" />{:else}<X class="size-3" />{/if}
                  Remove
                </button>
              {:else}
                <button
                  class="flex shrink-0 items-center gap-1.5 rounded-md bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary hover:bg-primary/20 transition-colors disabled:opacity-50"
                  onclick={() => addArtist(artist.name)}
                  disabled={isPending}
                >
                  {#if isPending}<Loader2 class="size-3 animate-spin" />{:else}<Plus class="size-3" />{/if}
                  Add
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {:else if query.trim() && !searching}
        <p class="mt-4 text-center text-sm text-muted-foreground">No artists found for "{query}"</p>
      {/if}
    </div>
  </section>

  <!-- Liked artists -->
  <section class="rounded-xl border border-border/70 bg-card">
    <div class="flex items-center justify-between border-b border-border/60 px-5 py-4">
      <div>
        <h2 class="font-semibold">Liked Artists</h2>
        <p class="mt-0.5 text-xs text-muted-foreground">Used to personalise Smart Shuffle and home page carousels.</p>
      </div>
      {#if !loadingLiked && likedArtists.length > 0}
        <span class="shrink-0 text-xs text-muted-foreground">{likedArtists.length} {likedArtists.length === 1 ? 'artist' : 'artists'}</span>
      {/if}
    </div>

    <div class="px-5 py-4">
      {#if loadingLiked}
        <div class="space-y-1">
          {#each Array(4) as _, i (i)}
            <div class="flex h-12 items-center gap-3">
              <div class="size-8 shrink-0 animate-pulse rounded-full bg-secondary"></div>
              <div class="h-3 w-32 animate-pulse rounded bg-secondary"></div>
            </div>
          {/each}
        </div>
      {:else if likedArtists.length === 0}
        <div class="flex flex-col items-center gap-2 py-10 text-muted-foreground">
          <Music2 class="size-9 opacity-25" />
          <p class="text-sm">No liked artists yet — search above to add some.</p>
        </div>
      {:else}
        <ul class="divide-y divide-border/40">
          {#each likedArtists as artist (artist.id)}
            {@const isPending = pending.has(artist.name.toLowerCase())}
            <li class="flex items-center gap-3 py-2.5">
              <div class="grid size-8 shrink-0 place-items-center rounded-full bg-primary/10 text-xs font-bold text-primary">
                {initials(artist.name)}
              </div>
              <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist.name}</span>
              <button
                class="shrink-0 rounded-md p-1.5 text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
                onclick={() => removeArtist(artist.name)}
                disabled={isPending}
                aria-label="Remove {artist.name}"
              >
                {#if isPending}<Loader2 class="size-3.5 animate-spin" />{:else}<X class="size-3.5" />{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </section>

</div>
