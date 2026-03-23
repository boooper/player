<script lang="ts">
  import { goto } from '$app/navigation';
  import { splitSongArtists } from '$lib/song-artists';

  let {
    artist,
    class: className = '',
    linkClass = ''
  }: {
    artist: string;
    class?: string;
    linkClass?: string;
  } = $props();

  const artists = $derived(splitSongArtists(artist));
</script>

<span class={className}>
  {#each artists as name, index (name)}
    <button
      class={linkClass}
      onclick={(event) => { event.stopPropagation(); goto(`/artist/${encodeURIComponent(name)}`); }}
    >{name}</button>{#if index < artists.length - 1}<span class="px-1">·</span>{/if}
  {/each}
</span>
