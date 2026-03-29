<script lang="ts">
  import { goto } from '$app/navigation';
  import { Disc3, Mic2, Album, Heart, ListMusic, Plus, Play, Pause } from '@lucide/svelte';
  import { subsonicPlaylists, isPlaying, playingFrom } from '$lib/stores/player';
  import { initials } from '$lib/utils';

  let { data } = $props<{ data: Record<string, never> }>();

  const sections = [
    { href: '/songs', label: 'Songs', icon: Disc3, color: 'from-violet-500/30 to-violet-500/10' },
    { href: '/album', label: 'Albums', icon: Album, color: 'from-blue-500/30 to-blue-500/10' },
    { href: '/artist', label: 'Artists', icon: Mic2, color: 'from-emerald-500/30 to-emerald-500/10' },
    { href: '/favorites', label: 'Favorites', icon: Heart, color: 'from-rose-500/30 to-rose-500/10' },
  ];
</script>

<!-- Quick nav tiles -->
<div class="mb-8 grid grid-cols-2 gap-3">
  {#each sections as s}
    <a
      href={s.href}
      class="flex items-center gap-3 rounded-2xl border border-white/[0.07] bg-gradient-to-br {s.color} p-4 active:opacity-80"
    >
      <s.icon class="size-5 shrink-0 text-white/70" />
      <span class="text-sm font-semibold">{s.label}</span>
    </a>
  {/each}
</div>

<!-- Playlists -->
<div>
  <div class="mb-3 flex items-center justify-between">
    <p class="text-[11px] font-semibold uppercase tracking-widest text-muted-foreground">Playlists</p>
    <button
      class="flex size-7 items-center justify-center rounded-full text-muted-foreground active:bg-white/10"
      onclick={() => goto('/settings')}
      aria-label="Manage playlists"
    >
      <Plus class="size-4" />
    </button>
  </div>

  {#if $subsonicPlaylists.length === 0}
    <div class="rounded-2xl border border-white/[0.06] bg-card/60 px-4 py-8 text-center">
      <ListMusic class="mx-auto mb-3 size-8 text-muted-foreground/40" />
      <p class="text-sm text-muted-foreground">No playlists yet</p>
    </div>
  {:else}
    <div class="overflow-hidden rounded-2xl border border-white/[0.06] bg-card/60">
      {#each $subsonicPlaylists as playlist, i (playlist.id)}
        <a
          href="/playlist/{encodeURIComponent(playlist.id)}"
          class="flex items-center gap-3 border-b border-white/[0.05] px-4 py-3 last:border-b-0 active:bg-white/[0.06]"
        >
          {#if playlist.coverArtUrl}
            <img
              class="size-11 shrink-0 rounded-xl object-cover shadow-md"
              src={playlist.coverArtUrl}
              alt={playlist.name}
            />
          {:else}
            <div class="grid size-11 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-white/10 to-white/5 text-xs font-bold text-muted-foreground">
              {initials(playlist.name)}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold">{playlist.name}</p>
            <p class="text-xs text-muted-foreground">
              {playlist.songCount ?? 0} {(playlist.songCount ?? 0) === 1 ? 'song' : 'songs'}
            </p>
          </div>
          {#if $playingFrom.href === `/playlist/${encodeURIComponent(playlist.id)}` && $isPlaying}
            <div class="shrink-0">
              <svg class="size-4 fill-primary text-primary" viewBox="0 0 24 24"><rect x="2" y="6" width="4" height="12" rx="1"/><rect x="9" y="3" width="4" height="18" rx="1"/><rect x="16" y="8" width="4" height="8" rx="1"/></svg>
            </div>
          {/if}
        </a>
      {/each}
    </div>
  {/if}
</div>
