<script lang="ts">
  import { PaneGroup } from 'paneforge';
  import { cn } from '$lib/utils';
  import type { PaneGroupProps } from 'paneforge';

  let { class: className, ...restProps }: PaneGroupProps = $props();
</script>

<PaneGroup
  class={cn('flex h-full w-full data-[direction=vertical]:flex-col', className)}
  {...restProps}
/>
