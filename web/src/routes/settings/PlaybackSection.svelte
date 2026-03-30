<script lang="ts">
  import { SlidersHorizontal, Power, AudioLines } from '@lucide/svelte';
  import { Slider } from '$lib/components/ui';
  import { toast } from 'svelte-sonner';
  import { DEFAULT_EQ_BANDS, EQ_PRESETS, findEqPresetId, normalizeEqBands, type EqBandValues, type EqPresetId } from '$lib/audio/equalizer';
  import EqGraph from '$lib/components/ui/eq-graph/eq-graph.svelte';
  import { setAutostart } from '$lib/servers';
  import * as Select from '$lib/components/ui/select';

  let {
    autostartEnabled = $bindable(false),
    crossfadeSeconds = $bindable(4),
    eqEnabled = $bindable(false),
    eqPreset = $bindable<EqPresetId>('flat'),
    eqBands = $bindable<EqBandValues>(DEFAULT_EQ_BANDS),
    title = 'Application',
    description = 'System-level application behaviour.',
    icon = 'power',
    showAutostart = true,
    showPlayback = true,
    showEqualizer = true,
  }: {
    autostartEnabled: boolean;
    crossfadeSeconds: number;
    eqEnabled: boolean;
    eqPreset: EqPresetId;
    eqBands: EqBandValues;
    title?: string;
    description?: string;
    icon?: 'power' | 'sound';
    showAutostart?: boolean;
    showPlayback?: boolean;
    showEqualizer?: boolean;
  } = $props();

  const HeaderIcon = $derived(icon === 'sound' ? AudioLines : Power);

  let autostartLoading = $state(false);

  async function toggleAutostart() {
    autostartLoading = true;
    try {
      const next = !autostartEnabled;
      await setAutostart(next);
      autostartEnabled = next;
    } catch {
      toast.error('Failed to update launch at login');
    } finally {
      autostartLoading = false;
    }
  }

  function updateEqPreset(nextPreset: string) {
    const preset = EQ_PRESETS.find((item) => item.id === nextPreset);
    if (!preset) return;
    eqPreset = preset.id;
    eqBands = [...preset.bands] as EqBandValues;
  }

  function onEqBandsChange(next: EqBandValues) {
    eqBands = next;
    eqPreset = findEqPresetId(eqBands);
  }
</script>

<section class="rounded-xl border border-border/70 bg-card">
  <div class="border-b border-border/60 px-5 py-4">
    <div class="flex items-center gap-2">
      <HeaderIcon class="size-4 text-muted-foreground" />
      <h2 class="font-semibold">{title}</h2>
    </div>
    <p class="mt-0.5 text-xs text-muted-foreground">{description}</p>
  </div>
  <div class="px-5 py-5">
    {#if showAutostart}
      <label class="flex cursor-pointer items-center gap-3" for="autostart-toggle">
        <input
          id="autostart-toggle"
          type="checkbox"
          class="sr-only"
          checked={autostartEnabled}
          onchange={toggleAutostart}
          disabled={autostartLoading}
        />
        <div
          aria-hidden="true"
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {autostartEnabled ? 'bg-primary' : 'bg-input'}"
        >
          <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {autostartEnabled ? 'translate-x-4' : ''}"></span>
        </div>
        <div>
          <p class="text-sm font-medium">Launch at login</p>
          <p class="text-xs text-muted-foreground">Automatically start Player when you log in.</p>
        </div>
      </label>
    {/if}

    {#if showPlayback}
      <div class="{showAutostart ? 'mt-5 border-t border-border/60 pt-5' : ''}">
        <div class="flex items-center justify-between gap-4">
          <div>
            <p class="text-sm font-medium">Crossfade</p>
            <p class="text-xs text-muted-foreground">Overlap duration between tracks during local playback.</p>
          </div>
          <span class="min-w-12 text-right text-sm font-semibold tabular-nums">{crossfadeSeconds.toFixed(1)}s</span>
        </div>


         <Slider
          class="mt-3 h-2 w-full cursor-pointer appearance-none rounded-full bg-input accent-primary"
          type="multiple"
          min={0}
          max={12}
          step={0.5}
          value={[crossfadeSeconds]}
          onValueChange={(value) => { crossfadeSeconds = value[0] ?? 0; }}
          aria-label="Crossfade duration"
        />
      </div>
    {/if}

    {#if showEqualizer}
      <div class="{showAutostart || showPlayback ? 'mt-5 border-t border-border/60 pt-5' : ''}">
        <div class="flex items-center gap-2">
          <SlidersHorizontal class="size-4 text-muted-foreground" />
          <div>
            <p class="text-sm font-medium">Equalizer</p>
            <p class="text-xs text-muted-foreground">Shape your sound with per-band gain adjustments.</p>
          </div>
        </div>

        <label class="mt-4 flex cursor-pointer items-center gap-3" for="eq-toggle">
          <input
            id="eq-toggle"
            type="checkbox"
            class="sr-only"
            checked={eqEnabled}
            onchange={() => { eqEnabled = !eqEnabled; }}
          />
          <div
            aria-hidden="true"
            class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {eqEnabled ? 'bg-primary' : 'bg-input'}"
          >
            <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {eqEnabled ? 'translate-x-4' : ''}"></span>
          </div>
          <div>
            <p class="text-sm font-medium">Enable equalizer</p>
            <p class="text-xs text-muted-foreground">Applies to local playback only, not casting.</p>
          </div>
        </label>

        <div class="mt-4">
          <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">EQ Preset</p>
          <Select.Root type="single" bind:value={eqPreset} onValueChange={updateEqPreset}>
            <Select.Trigger class="w-full">
              {EQ_PRESETS.find((preset) => preset.id === eqPreset)?.label ?? 'Select preset'}
            </Select.Trigger>
            <Select.Content>
              {#each EQ_PRESETS as preset (preset.id)}
                <Select.Item value={preset.id} label={preset.label} />
              {/each}
            </Select.Content>
          </Select.Root>
        </div>

        <div class="mt-4">
          <EqGraph bind:bands={eqBands} enabled={eqEnabled} onchange={onEqBandsChange} />
        </div>
      </div>
    {/if}
  </div>
</section>
