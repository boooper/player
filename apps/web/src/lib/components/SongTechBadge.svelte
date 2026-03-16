<script lang="ts">
  import { Cloud, HardDrive } from '@lucide/svelte';
  import { getExternalSourceLabel } from '$lib/external-source';

  let {
    id = null,
    cached = null,
    audioFormat = null,
    bitrateKbps = null,
    compact = false,
  }: {
    id?: string | null;
    cached?: boolean | null;
    audioFormat?: string | null;
    bitrateKbps?: number | null;
    compact?: boolean;
  } = $props();

  const sourceLabel = $derived(getExternalSourceLabel(id));
  const normalizedFormat = $derived(audioFormat?.trim().toUpperCase() ?? '');
  const bitrateLabel = $derived(
    bitrateKbps && bitrateKbps > 0 ? `${Math.round(bitrateKbps)} kbps` : ''
  );
</script>

{#if sourceLabel || cached !== null || normalizedFormat || bitrateLabel}
  <span
    class="inline-flex items-center gap-1.5 rounded-full border border-border/60 bg-white/[0.035] px-2 py-1 text-[10px] font-medium uppercase tracking-[0.16em] text-muted-foreground {compact ? 'px-1.5 py-0.5 text-[9px]' : ''}"
  >
    {#if sourceLabel}
      <span class="text-sky-200">{sourceLabel}</span>
    {/if}
    {#if cached !== null}
      {#if cached}
        <HardDrive class="size-3 text-emerald-400" />
      {:else}
        <Cloud class="size-3 text-muted-foreground/80" />
      {/if}
    {/if}
    {#if bitrateLabel}
      <span>{bitrateLabel}</span>
    {/if}
    {#if normalizedFormat}
      <span>{normalizedFormat}</span>
    {/if}
    {#if cached}
      <span class="size-1.5 rounded-full bg-emerald-400"></span>
    {/if}
  </span>
{/if}
