<script lang="ts">
  import { Badge } from '$lib/components/ui';
  import { getExternalSourceLabel } from '$lib/external-source';

  let { id, class: className = '' }: { id?: string | null; class?: string } = $props();

  const label = $derived(getExternalSourceLabel(id));
</script>

{#if label}
  <Badge variant="outline" class={`border-sky-500/30 bg-sky-500/10 text-sky-200 ${className}`.trim()}>
    {label}
  </Badge>
{/if}
