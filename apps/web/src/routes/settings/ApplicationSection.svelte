<script lang="ts">
  import { SlidersHorizontal, Power, AudioLines } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { DEFAULT_EQ_BANDS, EQ_FREQUENCIES, EQ_MAX_GAIN, EQ_MIN_GAIN, EQ_PRESETS, findEqPresetId, normalizeEqBands, type EqBandValues, type EqPresetId } from '$lib/audio/equalizer';
  import { setAutostart } from '$lib/api';
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

  const eqLabels = ['Sub', 'Bass', 'Mid', 'Presence', 'Air'];
  const eqPlotHeight = 188;

  let eqGraphEl = $state<HTMLDivElement | null>(null);
  let activeEqDragIndex = $state<number | null>(null);

  const eqFrequencyLabels = EQ_FREQUENCIES.map((frequency) =>
    frequency >= 1000 ? `${(frequency / 1000).toFixed(frequency % 1000 === 0 ? 0 : 1)}kHz` : `${frequency}Hz`
  );

  function updateEqPreset(nextPreset: string) {
    const preset = EQ_PRESETS.find((item) => item.id === nextPreset);
    if (!preset) return;
    eqPreset = preset.id;
    eqBands = [...preset.bands] as EqBandValues;
  }

  function updateEqBand(index: number, value: number) {
    const next = [...eqBands];
    next[index] = value;
    eqBands = normalizeEqBands(next);
    eqPreset = findEqPresetId(eqBands);
  }

  function gainToY(gain: number): number {
    const ratio = (EQ_MAX_GAIN - gain) / (EQ_MAX_GAIN - EQ_MIN_GAIN);
    return ratio * eqPlotHeight;
  }

  function yToGain(y: number): number {
    const clampedY = Math.max(0, Math.min(eqPlotHeight, y));
    const ratio = clampedY / eqPlotHeight;
    return EQ_MAX_GAIN - ratio * (EQ_MAX_GAIN - EQ_MIN_GAIN);
  }

  const eqPoints = $derived(
    EQ_FREQUENCIES.map((frequency, index) => {
      const x = (index / (EQ_FREQUENCIES.length - 1)) * 100;
      const y = gainToY(eqBands[index]);
      return {
        frequency,
        label: eqFrequencyLabels[index],
        gain: eqBands[index],
        x,
        y
      };
    })
  );

  const eqLinePoints = $derived(eqPoints.map((point) => `${point.x},${point.y}`).join(' '));
  const eqAreaPoints = $derived(`0,${eqPlotHeight} ${eqPoints.map((point) => `${point.x},${point.y}`).join(' ')} 100,${eqPlotHeight}`);

  function updateEqBandFromPointer(index: number, clientY: number) {
    if (!eqGraphEl) return;
    const rect = eqGraphEl.getBoundingClientRect();
    const nextY = ((clientY - rect.top) / rect.height) * eqPlotHeight;
    updateEqBand(index, Math.round(yToGain(nextY)));
  }

  function startEqDrag(index: number, event: PointerEvent) {
    if (!eqEnabled) return;
    activeEqDragIndex = index;
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    updateEqBandFromPointer(index, event.clientY);
  }

  function onEqPointerMove(event: PointerEvent) {
    if (!eqEnabled || activeEqDragIndex === null) return;
    updateEqBandFromPointer(activeEqDragIndex, event.clientY);
  }

  function stopEqDrag() {
    activeEqDragIndex = null;
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
            <p class="text-sm font-medium">Crossfade duration</p>
            <p class="text-xs text-muted-foreground">How long songs overlap during local playback.</p>
          </div>
          <span class="min-w-12 text-right text-sm font-semibold tabular-nums">{crossfadeSeconds.toFixed(1)}s</span>
        </div>
        <input
          class="mt-3 h-2 w-full cursor-pointer appearance-none rounded-full bg-input accent-primary"
          type="range"
          min="0"
          max="12"
          step="0.5"
          bind:value={crossfadeSeconds}
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
            <p class="text-xs text-muted-foreground">Apply EQ to local playback and use presets as a starting point.</p>
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
            <p class="text-sm font-medium">Enable EQ</p>
            <p class="text-xs text-muted-foreground">Uses Web Audio filters for local playback only.</p>
          </div>
        </label>

        <div class="mt-4">
          <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Preset</p>
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

        <div class="mt-4 rounded-[28px] border border-border/60 bg-[linear-gradient(180deg,rgba(255,255,255,0.03),rgba(255,255,255,0.01))] p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
          <div class="flex gap-4">
            <div class="flex w-11 shrink-0 flex-col justify-between py-5 text-xs font-semibold text-muted-foreground">
              <span>+12dB</span>
              <span>-12dB</span>
            </div>
            <div class="min-w-0 flex-1">
              <div
                class="relative h-60 w-full touch-none overflow-hidden rounded-[24px] border border-white/5 bg-[#232323] px-8 pt-5 pb-11 {eqEnabled ? '' : 'opacity-50'}"
                role="group"
                aria-label="Equalizer graph"
              >
                <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.05),transparent_55%),linear-gradient(180deg,rgba(255,255,255,0.04),rgba(0,0,0,0.14))]"></div>
                <div
                  bind:this={eqGraphEl}
                  class="absolute inset-x-8 top-5 bottom-11"
                  role="presentation"
                  onpointermove={onEqPointerMove}
                  onpointerup={stopEqDrag}
                  onpointercancel={stopEqDrag}
                >
                  <div class="pointer-events-none absolute left-0 right-0 top-1/2 h-px -translate-y-1/2 bg-primary/25"></div>

                  {#each eqPoints as point, index (point.frequency)}
                    <div
                      class="pointer-events-none absolute top-0 bottom-0 w-px bg-primary/15"
                      style={`left: ${point.x}%;`}
                    ></div>
                  {/each}

                  <svg
                    viewBox={`0 0 100 ${eqPlotHeight}`}
                    preserveAspectRatio="none"
                    class="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
                    aria-hidden="true"
                  >
                    <defs>
                      <linearGradient id="eq-fill-gradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stop-color="var(--primary)" stop-opacity="0.42" />
                        <stop offset="45%" stop-color="var(--primary)" stop-opacity="0.18" />
                        <stop offset="100%" stop-color="var(--primary)" stop-opacity="0" />
                      </linearGradient>
                    </defs>
                    <polygon points={eqAreaPoints} fill="url(#eq-fill-gradient)"></polygon>
                    <polyline
                      points={eqLinePoints}
                      fill="none"
                      stroke="var(--primary)"
                      stroke-width="2.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    ></polyline>
                  </svg>

                  {#each eqPoints as point, index (point.frequency)}
                    <button
                      type="button"
                      class="absolute flex h-8 w-8 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full transition disabled:cursor-not-allowed"
                      style={`left: ${point.x}%; top: ${(point.y / eqPlotHeight) * 100}%;`}
                      onpointerdown={(event) => startEqDrag(index, event)}
                      onpointerup={stopEqDrag}
                      disabled={!eqEnabled}
                      aria-label={`${eqLabels[index]} EQ ${point.label}`}
                    >
                      <span class="flex h-3.5 w-3.5 rounded-full border border-primary/70 bg-primary shadow-[0_0_0_5px_color-mix(in_srgb,var(--primary)_18%,transparent)]"></span>
                    </button>
                  {/each}
                </div>

                <div class="absolute inset-x-8 bottom-3 grid grid-cols-5 gap-2">
                  {#each eqPoints as point, index (point.frequency)}
                    <div class="text-center">
                      <p class="text-[15px] font-semibold tracking-[-0.01em] text-primary">{point.label}</p>
                      <p class="mt-1 text-[11px] font-medium text-primary/60">{eqBands[index] > 0 ? '+' : ''}{eqBands[index].toFixed(0)} dB</p>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</section>

<svelte:window onpointerup={stopEqDrag} onpointercancel={stopEqDrag} />
