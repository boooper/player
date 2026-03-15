<script lang="ts">
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
    <a
      href={`/artist/${encodeURIComponent(name)}`}
      class={linkClass}
      onclick={(event) => event.stopPropagation()}
    >{name}</a>{#if index < artists.length - 1}<span class="px-1">·</span>{/if}
  {/each}
</span>
