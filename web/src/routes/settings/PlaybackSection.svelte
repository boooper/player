<script lang="ts">
  import { SlidersHorizontal, Power, AudioLines } from '@lucide/svelte';
  import { Slider } from '$lib/components/ui';
  import { toast } from 'svelte-sonner';
  import { DEFAULT_EQ_BANDS, DEFAULT_EQ_FREQUENCIES, EQ_PRESETS, findEqPresetId, normalizeEqBands, type EqBandValues, type EqFrequencyValues, type EqPresetId } from '$lib/audio/equalizer';
  import EqGraph from '$lib/components/ui/eq-graph/eq-graph.svelte';
  import { setAutostart } from '$lib/servers';
  import * as Select from '$lib/components/ui/select';

  let {
    autostartEnabled = $bindable(false),
    crossfadeSeconds = $bindable(4),
    gaplessEnabled = $bindable(true),
    normalizationEnabled = $bindable(false),
    normalizationMode = $bindable<'lufs' | 'rms'>('lufs'),
    loudnessCompensationEnabled = $bindable(false),
    smartCrossfadeEnabled = $bindable(false),
    eqEnabled = $bindable(false),
    eqPreset = $bindable<EqPresetId>('flat'),
    eqFrequencies = $bindable<EqFrequencyValues>(DEFAULT_EQ_FREQUENCIES),
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
    gaplessEnabled: boolean;
    normalizationEnabled: boolean;
    normalizationMode: 'lufs' | 'rms';
    loudnessCompensationEnabled: boolean;
    smartCrossfadeEnabled: boolean;
    eqEnabled: boolean;
    eqPreset: EqPresetId;
    eqFrequencies: EqFrequencyValues;
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
  // Remember the last crossfade duration so switching away and back doesn't lose it.
  let lastCrossfadeSeconds = $state(crossfadeSeconds > 0 ? crossfadeSeconds : 4);

  const transitionMode = $derived(
    crossfadeSeconds > 0 ? 'crossfade' : gaplessEnabled ? 'gapless' : 'off'
  );

  function setTransitionMode(mode: string) {
    if (mode === 'crossfade') {
      gaplessEnabled = false;
      crossfadeSeconds = lastCrossfadeSeconds;
    } else if (mode === 'gapless') {
      gaplessEnabled = true;
      crossfadeSeconds = 0;
    } else {
      gaplessEnabled = false;
      crossfadeSeconds = 0;
    }
  }

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
    eqFrequencies = [...preset.frequencies] as EqFrequencyValues;
    eqBands = [...preset.bands] as EqBandValues;
  }

  function onEqChange(nextFrequencies: EqFrequencyValues, nextBands: EqBandValues) {
    eqFrequencies = nextFrequencies;
    eqBands = nextBands;
    eqPreset = findEqPresetId(eqBands, eqFrequencies);
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

        <!-- Track transitions -->
        <div class="flex items-center justify-between gap-4">
          <div>
            <p class="text-sm font-medium">Track transitions</p>
            <p class="text-xs text-muted-foreground">
              {#if transitionMode === 'crossfade'}Blend the end of one track into the next.
              {:else if transitionMode === 'gapless'}Seamlessly connect tracks with no silence.
              {:else}Tracks play one after another with a brief pause.{/if}
            </p>
          </div>
          <Select.Root type="single" value={transitionMode} onValueChange={setTransitionMode}>
            <Select.Trigger class="w-36 shrink-0">
              {transitionMode === 'crossfade' ? 'Crossfade' : transitionMode === 'gapless' ? 'Gapless' : 'Off'}
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="off">Off</Select.Item>
              <Select.Item value="gapless">Gapless</Select.Item>
              <Select.Item value="crossfade">Crossfade</Select.Item>
            </Select.Content>
          </Select.Root>
        </div>

        {#if transitionMode === 'crossfade'}
          <div class="mt-3">
            <div class="flex items-center justify-between gap-2">
              <p class="text-xs text-muted-foreground">Duration</p>
              <span class="text-xs font-semibold tabular-nums text-muted-foreground">{crossfadeSeconds.toFixed(1)}s</span>
            </div>
            <Slider
              class="mt-2 h-2 w-full cursor-pointer appearance-none rounded-full bg-input accent-primary"
              type="multiple"
              min={0.5}
              max={12}
              step={0.5}
              value={[crossfadeSeconds]}
              onValueChange={(value) => {
                crossfadeSeconds = value[0] ?? 0.5;
                lastCrossfadeSeconds = crossfadeSeconds;
              }}
              aria-label="Crossfade duration"
            />
          </div>

          <label class="mt-4 flex cursor-pointer items-center gap-3" for="smart-crossfade-toggle">
            <input
              id="smart-crossfade-toggle"
              type="checkbox"
              class="sr-only"
              checked={smartCrossfadeEnabled}
              onchange={() => { smartCrossfadeEnabled = !smartCrossfadeEnabled; }}
            />
            <div
              aria-hidden="true"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {smartCrossfadeEnabled ? 'bg-primary' : 'bg-input'}"
            >
              <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {smartCrossfadeEnabled ? 'translate-x-4' : ''}"></span>
            </div>
            <div>
              <p class="text-sm font-medium">Smart crossfade</p>
              <p class="text-xs text-muted-foreground">Trigger at the natural fade-out instead of a fixed time.</p>
            </div>
          </label>
        {/if}

        <!-- Loudness -->
        <div class="mt-5 border-t border-border/60 pt-5">
          <label class="flex cursor-pointer items-center gap-3" for="normalization-toggle">
            <input
              id="normalization-toggle"
              type="checkbox"
              class="sr-only"
              checked={normalizationEnabled}
              onchange={() => { normalizationEnabled = !normalizationEnabled; }}
            />
            <div
              aria-hidden="true"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {normalizationEnabled ? 'bg-primary' : 'bg-input'}"
            >
              <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {normalizationEnabled ? 'translate-x-4' : ''}"></span>
            </div>
            <div class="flex flex-1 items-center justify-between gap-4">
              <div>
                <p class="text-sm font-medium">Volume normalization</p>
                <p class="text-xs text-muted-foreground">Match loudness across tracks.</p>
              </div>
              {#if normalizationEnabled}
                <Select.Root
                  type="single"
                  value={normalizationMode}
                  onValueChange={(v) => { if (v === 'lufs' || v === 'rms') normalizationMode = v; }}
                >
                  <Select.Trigger class="w-44 shrink-0">
                    {normalizationMode === 'rms' ? '–18 dB RMS (legacy)' : '–14 LUFS (modern)'}
                  </Select.Trigger>
                  <Select.Content>
                    <Select.Item value="lufs">–14 LUFS <span class="text-muted-foreground">(modern)</span></Select.Item>
                    <Select.Item value="rms">–18 dB RMS <span class="text-muted-foreground">(legacy)</span></Select.Item>
                  </Select.Content>
                </Select.Root>
              {/if}
            </div>
          </label>

          <label class="mt-4 flex cursor-pointer items-center gap-3" for="loudness-compensation-toggle">
            <input
              id="loudness-compensation-toggle"
              type="checkbox"
              class="sr-only"
              checked={loudnessCompensationEnabled}
              onchange={() => { loudnessCompensationEnabled = !loudnessCompensationEnabled; }}
            />
            <div
              aria-hidden="true"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {loudnessCompensationEnabled ? 'bg-primary' : 'bg-input'}"
            >
              <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {loudnessCompensationEnabled ? 'translate-x-4' : ''}"></span>
            </div>
            <div>
              <p class="text-sm font-medium">Loudness compensation</p>
              <p class="text-xs text-muted-foreground">Boost bass and treble at low volumes (Fletcher-Munson).</p>
            </div>
          </label>
        </div>

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
          <EqGraph bind:frequencies={eqFrequencies} bind:bands={eqBands} enabled={eqEnabled} onchange={onEqChange} />
        </div>
      </div>
    {/if}
  </div>
</section>
