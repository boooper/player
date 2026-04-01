<script lang="ts">
  import { tweened } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import { EQ_MAX_GAIN, EQ_MIN_GAIN, normalizeEqBands, normalizeEqFrequencies, type EqBandValues, type EqFrequencyValues } from '$lib/audio/equalizer';

  let {
    frequencies = $bindable<EqFrequencyValues>(),
    bands = $bindable<EqBandValues>(),
    enabled = true,
    onchange,
  }: {
    frequencies: EqFrequencyValues;
    bands: EqBandValues;
    enabled?: boolean;
    onchange?: (frequencies: EqFrequencyValues, bands: EqBandValues) => void;
  } = $props();

  const plotHeight = 188;
  const pad = 10;
  const inner = plotHeight - pad * 2;

  // ── Per-band frequency caps ───────────────────────────────────────────────────

  // Min/max Hz each band's knob can reach
  const BAND_FREQ_RANGES: [number, number][] = [
    [20,    200],   // Sub
    [60,    400],   // Bass
    [150,   800],   // Low Mid
    [300,   1500],  // Mid
    [600,   3000],  // Upper Mid
    [1500,  7000],  // Presence
    [3000,  12000], // Brilliance
    [7000,  18000], // Air
    [12000, 20000], // Treble
  ];

  function formatFreq(hz: number): string {
    return hz >= 1000 ? `${(hz / 1000).toFixed(hz % 1000 === 0 ? 0 : 1)}kHz` : `${Math.round(hz)}Hz`;
  }

  // ── Graph helpers ────────────────────────────────────────────────────────────

  let graphEl = $state<HTMLDivElement | null>(null);

  function gainToY(gain: number): number {
    const ratio = (EQ_MAX_GAIN - gain) / (EQ_MAX_GAIN - EQ_MIN_GAIN);
    return pad + ratio * inner;
  }

  function yToGain(y: number): number {
    const clampedY = Math.max(pad, Math.min(plotHeight - pad, y));
    const ratio = (clampedY - pad) / inner;
    return EQ_MAX_GAIN - ratio * (EQ_MAX_GAIN - EQ_MIN_GAIN);
  }

  // Graph x positions are fixed by band index — not tied to actual Hz
  const BAND_COUNT = 9;
  const points = $derived(
    frequencies.map((freq, index) => {
      const x = (index / (BAND_COUNT - 1)) * 100;
      const y = gainToY(bands[index]);
      return { freq, gain: bands[index], x, y };
    })
  );

  // Tweened y-positions for the SVG line — animates on preset changes, instant during drag
  const displayYs = tweened<number[]>(
    frequencies.map((_, index) => gainToY(bands[index])),
    { duration: 250, easing: cubicOut }
  );

  $effect(() => {
    const ys = points.map((p) => p.y);
    displayYs.set(ys, { duration: activeDragIndex !== null ? 0 : 250 });
  });

  const linePoints = $derived(
    points.map((p, i) => `${p.x},${$displayYs[i] ?? p.y}`).join(' ')
  );

  function updateGain(index: number, gain: number) {
    const nextBands = [...bands] as EqBandValues;
    nextBands[index] = Math.round(Math.max(EQ_MIN_GAIN, Math.min(EQ_MAX_GAIN, gain)));
    bands = normalizeEqBands(nextBands) as EqBandValues;
    onchange?.(frequencies, bands);
  }

  function updateFreq(index: number, freq: number) {
    const [minHz, maxHz] = BAND_FREQ_RANGES[index];
    const nextFreqs = [...frequencies] as EqFrequencyValues;
    nextFreqs[index] = Math.max(minHz, Math.min(maxHz, freq));
    frequencies = normalizeEqFrequencies(nextFreqs) as EqFrequencyValues;
    onchange?.(frequencies, bands);
  }

  // ── Gain drag (graph circles, Y axis only) ───────────────────────────────────

  let activeDragIndex = $state<number | null>(null);

  function startDrag(index: number, event: PointerEvent) {
    if (!enabled) return;
    activeDragIndex = index;
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    updateGainFromPointer(index, event.clientY);
  }

  function updateGainFromPointer(index: number, clientY: number) {
    if (!graphEl) return;
    const rect = graphEl.getBoundingClientRect();
    const relY = ((clientY - rect.top) / rect.height) * plotHeight;
    updateGain(index, yToGain(relY));
  }

  function onPointerMove(event: PointerEvent) {
    if (!enabled || activeDragIndex === null) return;
    updateGainFromPointer(activeDragIndex, event.clientY);
  }

  function stopDrag() {
    activeDragIndex = null;
  }

  // ── Knob helpers (frequency control) ─────────────────────────────────────────

  // Knob arc: -135° to +135° from 12-o'clock, total 270°
  const KNOB_START = -135;
  const KNOB_RANGE = 270;
  const R = 16;
  const CX = 20;
  const CY = 20;

  // Map freq within this band's [minHz, maxHz] to a knob angle, using log scale
  function freqToAngle(index: number, freq: number): number {
    const [minHz, maxHz] = BAND_FREQ_RANGES[index];
    const ratio = (Math.log(Math.max(minHz, freq)) - Math.log(minHz)) / (Math.log(maxHz) - Math.log(minHz));
    return KNOB_START + Math.max(0, Math.min(1, ratio)) * KNOB_RANGE;
  }

  function polarToXY(angleDeg: number, r: number): { x: number; y: number } {
    const rad = (angleDeg - 90) * (Math.PI / 180);
    return { x: CX + r * Math.cos(rad), y: CY + r * Math.sin(rad) };
  }

  function arcPath(startAngle: number, endAngle: number, r: number): string {
    const s = polarToXY(startAngle, r);
    const e = polarToXY(endAngle, r);
    const large = Math.abs(endAngle - startAngle) > 180 ? 1 : 0;
    const sweep = endAngle > startAngle ? 1 : 0;
    return `M ${s.x.toFixed(2)} ${s.y.toFixed(2)} A ${r} ${r} 0 ${large} ${sweep} ${e.x.toFixed(2)} ${e.y.toFixed(2)}`;
  }

  function knobTrackPath(): string {
    return arcPath(KNOB_START, KNOB_START + KNOB_RANGE, R);
  }

  function knobFillPath(index: number, freq: number): string {
    const angle = freqToAngle(index, freq);
    if (Math.abs(angle - KNOB_START) < 1) return '';
    return arcPath(KNOB_START, angle, R);
  }

  let activeKnobIndex = $state<number | null>(null);
  let knobPrevY = $state(0);

  function startKnobDrag(index: number, event: PointerEvent) {
    if (!enabled) return;
    activeKnobIndex = index;
    knobPrevY = event.clientY;
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  }

  function onKnobPointerMove(event: PointerEvent) {
    if (!enabled || activeKnobIndex === null) return;
    const dy = knobPrevY - event.clientY; // up = higher freq
    knobPrevY = event.clientY;
    const [minHz, maxHz] = BAND_FREQ_RANGES[activeKnobIndex];
    const logMin = Math.log(minHz);
    const logMax = Math.log(maxHz);
    const currentRatio = (Math.log(Math.max(minHz, frequencies[activeKnobIndex])) - logMin) / (logMax - logMin);
    const nextRatio = Math.max(0, Math.min(1, currentRatio + dy / 160));
    updateFreq(activeKnobIndex, Math.exp(logMin + nextRatio * (logMax - logMin)));
  }

  function stopKnobDrag() {
    activeKnobIndex = null;
  }
</script>

<div>

      <!-- Graph -->
      <div
        class="relative h-48 w-full touch-none overflow-hidden rounded-t-[24px] border border-b-0 border-border/40 {enabled ? '' : 'opacity-50'}"
        role="group"
        aria-label="Equalizer graph"
      >
        <div class="pointer-events-none absolute inset-0" style="background: radial-gradient(circle at top, color-mix(in srgb, var(--foreground) 5%, transparent) 0%, transparent 55%), linear-gradient(180deg, color-mix(in srgb, var(--foreground) 4%, transparent) 0%, rgba(0,0,0,0.14) 100%);"></div>
        <!-- dB labels inside graph -->
        <span class="pointer-events-none absolute top-2 left-3 text-[10px] font-semibold text-muted-foreground/60">+12dB</span>
        <span class="pointer-events-none absolute bottom-2 left-3 text-[10px] font-semibold text-muted-foreground/60">-12dB</span>
        <div
          bind:this={graphEl}
          class="absolute inset-x-4 top-5 bottom-2"
          role="presentation"
          onpointermove={onPointerMove}
          onpointerup={stopDrag}
          onpointercancel={stopDrag}
        >
          <div class="pointer-events-none absolute left-0 right-0 top-1/2 h-px -translate-y-1/2 bg-primary/25"></div>

          {#each points as point (point.freq)}
            <div class="pointer-events-none absolute top-0 bottom-0 w-px bg-primary/15" style={`left: ${point.x}%;`}></div>
          {/each}

          <svg
            viewBox={`0 0 100 ${plotHeight}`}
            preserveAspectRatio="none"
            class="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
            aria-hidden="true"
          >
            <polyline
              points={linePoints}
              fill="none"
              stroke="var(--primary)"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              vector-effect="non-scaling-stroke"
            ></polyline>
          </svg>

          {#each points as point, index (point.freq)}
            {@const isActive = activeDragIndex === index}
            <button
              type="button"
              class="absolute flex h-8 w-8 -translate-x-1/2 -translate-y-1/2 cursor-move items-center justify-center rounded-full disabled:cursor-not-allowed {activeDragIndex === null ? 'transition-[top] duration-200 ease-out' : ''}"
              style={`left: ${point.x}%; top: ${(point.y / plotHeight) * 100}%;`}
              onpointerdown={(event) => startDrag(index, event)}
              onpointerup={stopDrag}
              disabled={!enabled}
              aria-label={`Band ${index + 1} EQ`}
            >
              <span class="flex rounded-full border border-primary/70 bg-primary transition-[height,width,box-shadow] duration-150 {isActive ? 'h-4 w-4 shadow-[0_0_0_7px_color-mix(in_srgb,var(--primary)_28%,transparent)]' : 'h-3.5 w-3.5 shadow-[0_0_0_5px_color-mix(in_srgb,var(--primary)_18%,transparent)]'}"></span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Knobs -->
      <div class="overflow-x-auto rounded-b-[24px] border border-t-0 border-border/40 bg-card/60 {enabled ? '' : 'opacity-50'}">
      <div class="grid grid-cols-9 min-w-[480px] px-8 py-4">
        {#each points as point, index (point.freq)}
          {@const fillPath = knobFillPath(index, point.freq)}
          <div class="flex flex-col items-center gap-1">
            <p class="text-[9px] font-medium text-muted-foreground/70 leading-tight tabular-nums">{formatFreq(point.freq)}</p>
            <button
              type="button"
              class="touch-none cursor-ns-resize disabled:cursor-not-allowed"
              onpointerdown={(e) => startKnobDrag(index, e)}
              disabled={!enabled}
              aria-label={`Band ${index + 1} ${formatFreq(point.freq)}`}
            >
              <svg width="36" height="36" viewBox="0 0 40 40" aria-hidden="true">
                <!-- Track -->
                <path
                  d={knobTrackPath()}
                  fill="none"
                  stroke="var(--primary)"
                  stroke-opacity="0.15"
                  stroke-width="3"
                  stroke-linecap="round"
                />
                <!-- Fill -->
                {#if fillPath}
                  <path
                    d={fillPath}
                    fill="none"
                    stroke="var(--primary)"
                    stroke-width="3"
                    stroke-linecap="round"
                  />
                {/if}
                <!-- Indicator dot -->
                {#each [freqToAngle(index, point.freq)] as angle}
                  {@const dot = polarToXY(angle, R)}
                  <circle cx={dot.x} cy={dot.y} r="2.5" fill="var(--primary)" />
                {/each}
              </svg>
            </button>
            <p class="text-[10px] font-semibold tabular-nums text-primary">{point.gain > 0 ? '+' : ''}{point.gain.toFixed(0)} dB</p>
          </div>
        {/each}
      </div>
      </div>

</div>

<svelte:window
  onpointermove={onKnobPointerMove}
  onpointerup={() => { stopDrag(); stopKnobDrag(); }}
  onpointercancel={() => { stopDrag(); stopKnobDrag(); }}
/>
