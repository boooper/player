<script lang="ts">
  import type { Snippet } from 'svelte';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator
  } from '$lib/components/ui/dropdown-menu';
  import { Button } from '$lib/components/ui';
  import { Shuffle, Sparkles, Mic2, Disc3 } from '@lucide/svelte';
  import { shuffleEnabled, smartShuffleMode } from '$lib/stores/player';
  import { formatSongArtists } from '$lib/song-artists';

  type TrackLike = {
    artist: string;
    album?: string;
    albumId?: string | null;
  } | null;

  type Props = {
    currentTrack: TrackLike;
    smartShuffleFetching: boolean;
    shuffleButtonClass: string;
    onActivateShuffle: () => void;
    onActivateSmartShuffle: () => void;
    onDeactivateShuffle: () => void;
    onShuffleArtist: () => void;
    onShuffleAlbum: () => void;
  };

  let {
    currentTrack,
    smartShuffleFetching,
    shuffleButtonClass,
    onActivateShuffle,
    onActivateSmartShuffle,
    onDeactivateShuffle,
    onShuffleArtist,
    onShuffleAlbum
  }: Props = $props();
</script>

{#snippet menuItem(icon: Snippet, label: string, desc: string, active: boolean, onclick: () => void, disabled = false)}
  <DropdownMenuItem {onclick} {disabled} class="gap-3 {active ? 'text-primary' : ''}">
    {@render icon()}
    <div>
      <p class="font-medium">{label}</p>
      <p class="truncate max-w-36 text-xs text-muted-foreground">{desc}</p>
    </div>
    {#if active}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
  </DropdownMenuItem>
{/snippet}

{#snippet shuffleIcon()}<Shuffle class="size-4 shrink-0" />{/snippet}
{#snippet smartShuffleIcon()}<Sparkles class="size-4 shrink-0 transition-opacity {smartShuffleFetching ? 'animate-pulse' : ''}" />{/snippet}
{#snippet mic2Icon()}<Mic2 class="size-4 shrink-0" />{/snippet}
{#snippet disc3Icon()}<Disc3 class="size-4 shrink-0" />{/snippet}

<DropdownMenu>
  <DropdownMenuTrigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="ghost"
        size="icon-sm"
        class={`player-transport-button ${shuffleButtonClass}`}
        aria-label="Shuffle options"
        title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
      >
        {#if $smartShuffleMode}
          <Sparkles class="size-3.5 transition-opacity {smartShuffleFetching ? 'animate-pulse' : ''}" />
        {:else}
          <Shuffle class="size-3.5" />
        {/if}
      </Button>
    {/snippet}
  </DropdownMenuTrigger>

  <DropdownMenuContent side="top" align="start" class="min-w-56">
    {@render menuItem(shuffleIcon, 'Shuffle', 'Play queue in random order', $shuffleEnabled && !$smartShuffleMode, onActivateShuffle)}
    {@render menuItem(smartShuffleIcon, 'Smart Shuffle', 'Weaves in Last.fm recommendations', $smartShuffleMode, onActivateSmartShuffle)}

    {#if currentTrack}
      <DropdownMenuSeparator />
      {@render menuItem(mic2Icon, 'Shuffle Artist', formatSongArtists(currentTrack.artist), false, onShuffleArtist)}
      {@render menuItem(disc3Icon, 'Shuffle Album', currentTrack.album ?? '', false, onShuffleAlbum, !currentTrack.albumId)}
    {/if}

    <DropdownMenuSeparator />
    <DropdownMenuItem onclick={onDeactivateShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
      <span class="flex size-4 shrink-0 items-center justify-center text-xs font-bold">—</span>
      <div>
        <p class="font-medium">Off</p>
        <p class="text-xs text-muted-foreground">Play in order</p>
      </div>
      {#if !$shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
    </DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>
