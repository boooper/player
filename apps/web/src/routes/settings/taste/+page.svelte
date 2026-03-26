<script lang="ts">
  import { onMount } from 'svelte';
  import { Heart, Search, X, Loader2, Plus, Music2 } from '@lucide/svelte';
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
      const next = new Set(pending);
      next.delete(key);
      pending = next;
    }
  }

  async function removeArtist(name: string) {
    const key = name.toLowerCase();
    pending = new Set([...pending, key]);
    try {
      await removeLikedArtist(name);
      likedArtists = likedArtists.filter((a) => a.name.toLowerCase() !== key);
    } finally {
      const next = new Set(pending);
      next.delete(key);
      pending = next;
    }
  }

  onMount(async () => {
    try {
      likedArtists = await fetchLikedArtists();
    } finally {
      loadingLiked = false;
    }
  });
</script>

<div class="space-y-8">
  <!-- Header description -->
  <div class="page-section p-5">
    <div class="flex items-start gap-3">
      <Heart class="mt-0.5 size-5 shrink-0 text-primary" />
      <div>
        <h2 class="text-base font-semibold">Your Taste</h2>
        <p class="mt-0.5 text-sm text-muted-foreground">
          Artists you add here are used to personalise Smart Shuffle and the home page carousels. Search your library or any artist on Last.FM.
        </p>
      </div>
    </div>
  </div>

  <!-- Search -->
  <div class="page-section space-y-4 p-5">
    <h3 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Add Artists</h3>

    <div class="relative">
      <Search class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <input
        type="text"
        placeholder="Search artists…"
        class="w-full rounded-xl border border-border bg-secondary/40 py-2.5 pl-9 pr-4 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/40"
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
        >
          <X class="size-4" />
        </button>
      {/if}
    </div>

    {#if searchResults.length > 0}
      <ul class="divide-y divide-border rounded-xl border border-border overflow-hidden">
        {#each searchResults as artist (artist.id)}
          {@const isLiked = likedNames.has(artist.name.toLowerCase())}
          {@const isPending = pending.has(artist.name.toLowerCase())}
          <li class="flex items-center gap-3 bg-secondary/20 px-4 py-2.5 first:rounded-t-xl last:rounded-b-xl hover:bg-secondary/40 transition-colors">
            {#if artist.imageUrl}
              <img src={artist.imageUrl} alt={artist.name} class="size-9 shrink-0 rounded-full object-cover" />
            {:else}
              <div class="grid size-9 shrink-0 place-items-center rounded-full bg-secondary text-xs font-bold text-muted-foreground">
                {initials(artist.name)}
              </div>
            {/if}
            <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist.name}</span>
            {#if isLiked}
              <button
                class="flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
                onclick={() => removeArtist(artist.name)}
                disabled={isPending}
              >
                {#if isPending}<Loader2 class="size-3 animate-spin" />{:else}<X class="size-3" />{/if}
                Remove
              </button>
            {:else}
              <button
                class="flex shrink-0 items-center gap-1.5 rounded-lg bg-primary/10 px-3 py-1.5 text-xs font-semibold text-primary hover:bg-primary/20 transition-colors disabled:opacity-50"
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
      <p class="py-2 text-center text-sm text-muted-foreground">No artists found for "{query}"</p>
    {/if}
  </div>

  <!-- Liked artists list -->
  <div class="page-section space-y-4 p-5">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Liked Artists</h3>
      {#if !loadingLiked}
        <span class="text-xs text-muted-foreground">{likedArtists.length} {likedArtists.length === 1 ? 'artist' : 'artists'}</span>
      {/if}
    </div>

    {#if loadingLiked}
      <div class="space-y-2">
        {#each Array(4) as _, i (i)}
          <div class="flex h-14 items-center gap-3 px-1">
            <div class="size-9 shrink-0 animate-pulse rounded-full bg-secondary"></div>
            <div class="h-3.5 w-36 animate-pulse rounded bg-secondary"></div>
          </div>
        {/each}
      </div>
    {:else if likedArtists.length === 0}
      <div class="flex flex-col items-center gap-2 py-10 text-muted-foreground">
        <Music2 class="size-10 opacity-30" />
        <p class="text-sm">No liked artists yet. Search above to add some.</p>
      </div>
    {:else}
      <ul class="divide-y divide-border rounded-xl border border-border overflow-hidden">
        {#each likedArtists as artist (artist.id)}
          {@const isPending = pending.has(artist.name.toLowerCase())}
          <li class="flex items-center gap-3 bg-secondary/20 px-4 py-2.5 first:rounded-t-xl last:rounded-b-xl hover:bg-secondary/40 transition-colors">
            <div class="grid size-9 shrink-0 place-items-center rounded-full bg-primary/10 text-xs font-bold text-primary">
              {initials(artist.name)}
            </div>
            <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist.name}</span>
            <button
              class="flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
              onclick={() => removeArtist(artist.name)}
              disabled={isPending}
              aria-label="Remove {artist.name}"
            >
              {#if isPending}
                <Loader2 class="size-3.5 animate-spin" />
              {:else}
                <X class="size-3.5" />
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
