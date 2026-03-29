<script lang="ts">
  import { Volume, Volume1, Volume2, VolumeX } from '@lucide/svelte';
  import { Slider } from '$lib/components/ui';
  import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';
  import { volume } from '$lib/stores/player';
  import { saveVolume } from '$lib/servers';
  import { createPlayerVolumeController } from '$lib/components/player/player-volume-controller.svelte';

  type Props = {
    castActive?: boolean;
    castVolume?: number | null;
  };

  let {
    castActive = false,
    castVolume = null
  }: Props = $props();

  const controller = createPlayerVolumeController({
    getCastActive: () => castActive,
    getCastVolume: () => castVolume,
    getLocalVolume: () => $volume
  });

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let lastQueuedVolume: number | null = null;

  const effectiveVolume = $derived(
    castActive ? (castVolume ?? $volume) : $volume
  );

  const isMuted = $derived(effectiveVolume <= 0.01);
  const volumePct = $derived(effectiveVolume * 100);

  $effect(() => {
    controller.syncFromExternalVolume(effectiveVolume);
  });

  $effect(() => {
    if (castActive) return;

    const current = $volume;

    if (lastQueuedVolume !== null && Math.abs(lastQueuedVolume - current) < 0.0001) {
      return;
    }

    if (saveTimer) clearTimeout(saveTimer);

    lastQueuedVolume = current;
    saveTimer = setTimeout(() => {
      saveVolume(current).catch((err) => {
        console.error('saveVolume failed', err);
      });
    }, 400);

    return () => {
      if (saveTimer) clearTimeout(saveTimer);
    };
  });

  function wheelVolume(node: HTMLElement) {
    const handler = (event: WheelEvent) => {
      controller.onWheel(event, effectiveVolume);
    };

    node.addEventListener('wheel', handler, { passive: false });

    return {
      destroy() {
        node.removeEventListener('wheel', handler);
        if (saveTimer) clearTimeout(saveTimer);
      }
    };
  }
</script>

<div
  class="player-volume-group"
  role="group"
  aria-label="Volume"
  use:wheelVolume
>
  <Tooltip>
    <TooltipTrigger>
      {#snippet child({ props })}
        <button {...props} type="button" onclick={() => controller.toggleMute(effectiveVolume)} aria-label={isMuted ? 'Unmute' : 'Mute'} class="player-volume-button">
          {#if isMuted}<VolumeX class="size-[18px]" />{:else if volumePct < 33}<Volume class="size-[18px]" />{:else if volumePct < 66}<Volume1 class="size-[18px]" />{:else}<Volume2 class="size-[18px]" />{/if}
        </button>
      {/snippet}
    </TooltipTrigger>
    <TooltipContent side="top" sideOffset={6}>{isMuted ? 'Unmute' : 'Mute'}</TooltipContent>
  </Tooltip>

  <div class="player-volume-shell">
    <Slider
      class="player-volume-slider"
      type="multiple"
      bind:value={controller.volVal}
      min={0}
      max={100}
      step={1}
      onpointerdown={controller.beginDrag}
      onValueChange={controller.changeVolume}
      onValueCommit={controller.commitVolume}
      aria-label="Volume"
    />
  </div>
</div>

<style>
  .player-volume-group {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    max-width: 11rem;
    align-items: center;
    gap: 0.5rem;
  }

  .player-volume-button {
    flex-shrink: 0;
    color: hsl(var(--muted-foreground));
    transition:
      transform 180ms ease,
      color 180ms ease,
      opacity 180ms ease;
  }

  .player-volume-button:hover {
    transform: translateY(-1px);
    color: hsl(var(--foreground));
  }

  .player-volume-shell {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;
  }

  :global(.player-volume-slider) {
    position: relative;
    z-index: 2;
    min-height: 1rem;
  }

  :global(.player-volume-slider[data-slot='slider']) {
    min-height: 1rem;
  }

  :global(.player-volume-slider [data-slot='slider-track']) {
    width: 100%;
  }
</style>