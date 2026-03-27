<script lang="ts">
  import type { Snippet } from 'svelte';
  import { User } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';

  let { name, children, triggerClass }: {
    name: string;
    children: Snippet;
    triggerClass?: string;
  } = $props();
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class={triggerClass ?? 'block w-full'}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    <ContextMenu.Item onclick={() => goto(`/artist/${encodeURIComponent(name)}`)}><User />Go to artist</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>
