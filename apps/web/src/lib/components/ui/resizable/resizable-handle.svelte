<script lang="ts">
  import { GripVertical } from '@lucide/svelte';
  import { PaneResizer } from 'paneforge';
  import { cn } from '$lib/utils';
  import type { PaneResizerProps } from 'paneforge';

  let {
    class: className,
    withHandle = false,
    ...restProps
  }: PaneResizerProps & {
    withHandle?: boolean;
  } = $props();
</script>

<PaneResizer
  class={cn(
    'relative flex w-1 shrink-0 items-center justify-center bg-transparent transition-colors hover:bg-white/10 data-[active=pointer]:bg-white/10 after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-border/40',
    className
  )}
  {...restProps}
>
  {#if withHandle}
    <div class="z-10 flex h-8 w-4 items-center justify-center rounded-full border border-border/70 bg-background/90 shadow-sm">
      <GripVertical class="size-3 text-muted-foreground" />
    </div>
  {/if}
</PaneResizer>
