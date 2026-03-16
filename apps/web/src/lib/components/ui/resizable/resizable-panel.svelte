<script lang="ts">
  import { Pane } from 'paneforge';
  import { cn } from '$lib/utils';
  import type { PaneProps } from 'paneforge';

  let { class: className, ...restProps }: PaneProps = $props();
</script>

<Pane class={cn('min-w-0 min-h-0', className)} {...restProps} />
