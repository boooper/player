<script lang="ts">
  import type { Album } from '$lib/servers';
  import { AlbumCard } from '$lib/components/media';
  import {
    Carousel,
    CarouselContent,
    CarouselItem,
    CarouselNext,
    CarouselPrevious,
    CarouselSeeAll,
  } from '$lib/components/ui/carousel';

  let {
    title,
    allItems,
    expanded,
    loading = false,
    onExpand,
    onCollapse,
  }: {
    title: string;
    allItems: Album[];
    expanded: boolean;
    loading?: boolean;
    onExpand: () => void;
    onCollapse: () => void;
  } = $props();
</script>

<section class="page-section">
  <div class="mb-5 flex items-center justify-between">
    <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">{title}</h2>
    {#if expanded}
      <button
        class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
        onclick={onCollapse}
      >Show Less</button>
    {/if}
  </div>

  {#if loading}
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 xl:grid-cols-5">
      {#each Array(5) as _, i (i)}
        <div class="space-y-3">
          <div class="aspect-square animate-pulse rounded-[24px] bg-white/6"></div>
          <div class="h-4 w-3/4 animate-pulse rounded-full bg-white/6"></div>
          <div class="h-3 w-1/2 animate-pulse rounded-full bg-white/5"></div>
        </div>
      {/each}
    </div>
  {:else if expanded}
    <div class="section-enter grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
      {#each allItems as album (album.id)}
        <AlbumCard {album} />
      {/each}
    </div>
  {:else}
    <div class="section-enter">
      <Carousel opts={{ align: 'start', dragFree: true }}>
        <CarouselContent>
          {#each allItems as album (album.id)}
            <CarouselItem class="basis-[36%] sm:basis-[24%] lg:basis-[18%] xl:basis-[14%]">
              <AlbumCard {album} />
            </CarouselItem>
          {/each}
        </CarouselContent>
        <CarouselPrevious class="carousel-nav" />
        <CarouselNext class="carousel-nav" />
        {#if allItems.length > 12}
          <CarouselSeeAll onclick={onExpand} />
        {/if}
      </Carousel>
    </div>
  {/if}
</section>

<style>
  .section-enter {
    animation: section-enter 240ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
  }

  @keyframes section-enter {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
