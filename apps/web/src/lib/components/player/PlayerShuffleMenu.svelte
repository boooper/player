<script lang="ts">
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
    <DropdownMenuItem onclick={onActivateShuffle} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
      <Shuffle class="size-4 shrink-0" />
      <div>
        <p class="font-medium">Shuffle</p>
        <p class="text-xs text-muted-foreground">Play queue in random order</p>
      </div>
      {#if $shuffleEnabled && !$smartShuffleMode}
        <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
      {/if}
    </DropdownMenuItem>

    <DropdownMenuItem onclick={onActivateSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
      <Sparkles class="size-4 shrink-0" />
      <div>
        <p class="font-medium">Smart Shuffle</p>
        <p class="text-xs text-muted-foreground">Weaves in Last.fm recommendations</p>
      </div>
      {#if $smartShuffleMode}
        <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
      {/if}
    </DropdownMenuItem>

    {#if currentTrack}
      <DropdownMenuSeparator />
      <DropdownMenuItem onclick={onShuffleArtist} class="gap-3">
        <Mic2 class="size-4 shrink-0" />
        <div>
          <p class="font-medium">Shuffle Artist</p>
          <p class="truncate max-w-36 text-xs text-muted-foreground">{formatSongArtists(currentTrack.artist)}</p>
        </div>
      </DropdownMenuItem>

      <DropdownMenuItem onclick={onShuffleAlbum} disabled={!currentTrack.albumId} class="gap-3">
        <Disc3 class="size-4 shrink-0" />
        <div>
          <p class="font-medium">Shuffle Album</p>
          <p class="truncate max-w-36 text-xs text-muted-foreground">{currentTrack.album}</p>
        </div>
      </DropdownMenuItem>
    {/if}

    <DropdownMenuSeparator />

    <DropdownMenuItem onclick={onDeactivateShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
      <span class="size-4 shrink-0 flex items-center justify-center text-xs font-bold">—</span>
      <div>
        <p class="font-medium">Off</p>
        <p class="text-xs text-muted-foreground">Play in order</p>
      </div>
      {#if !$shuffleEnabled && !$smartShuffleMode}
        <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
      {/if}
    </DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>