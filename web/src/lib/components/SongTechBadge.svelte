<script lang="ts">
  import { Disc3 } from '@lucide/svelte';
  import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';

  let {
    cached = null,
    audioFormat = null,
    bitrateKbps = null,
    compact = false,
  }: {
    cached?: boolean | null;
    audioFormat?: string | null;
    bitrateKbps?: number | null;
    compact?: boolean;
  } = $props();

  const normalizedFormat = $derived(audioFormat?.trim().toUpperCase() ?? '');
  const bitrateLabel = $derived(
    bitrateKbps && bitrateKbps > 0 ? `${Math.round(bitrateKbps)} kbps` : ''
  );
  const hasAnyInfo = $derived(!!normalizedFormat || !!bitrateLabel || cached !== null);
</script>

{#if hasAnyInfo}
  <Tooltip>
    <TooltipTrigger>
      {#snippet child({ props })}
        <span
          {...props}
          class="shrink-0 inline-flex cursor-default items-center justify-center rounded-full border p-0.5 {cached
            ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'
            : 'border-border/50 bg-transparent text-muted-foreground/50'}"
        >
          <Disc3 class="size-2.5" />
        </span>
      {/snippet}
    </TooltipTrigger>
    <TooltipContent side="top" sideOffset={6}>
      <div class="flex flex-col gap-0.5">
        {#if normalizedFormat || bitrateLabel}
          <span class="font-medium">{[normalizedFormat, bitrateLabel].filter(Boolean).join(' · ')}</span>
        {/if}
        {#if cached}
          <span class="text-popover-foreground/60">Cached locally</span>
        {/if}
      </div>
    </TooltipContent>
  </Tooltip>
{/if}
