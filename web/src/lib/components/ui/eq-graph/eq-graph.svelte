<script lang="ts">
  import { EQ_FREQUENCIES, EQ_MAX_GAIN, EQ_MIN_GAIN, normalizeEqBands, type EqBandValues } from '$lib/audio/equalizer';

  let {
    bands = $bindable<EqBandValues>(),
    enabled = true,
    onchange,
  }: {
    bands: EqBandValues;
    enabled?: boolean;
    onchange?: (bands: EqBandValues) => void;
  } = $props();

  const labels = ['Sub', 'Bass', 'Low Mid', 'Mid', 'Upper Mid', 'Presence', 'Brilliance', 'Air', 'Treble'];
  const plotHeight = 188;
  const pad = 10;
  const inner = plotHeight - pad * 2;

  const frequencyLabels = EQ_FREQUENCIES.map((f) =>
    f >= 1000 ? `${(f / 1000).toFixed(f % 1000 === 0 ? 0 : 1)}kHz` : `${f}Hz`
  );

  let graphEl = $state<HTMLDivElement | null>(null);
  let activeDragIndex = $state<number | null>(null);
  let activeKnobIndex = $state<number | null>(null);
  let knobDragStartY = $state(0);
  let knobDragStartGain = $state(0);

  // ── Graph helpers ────────────────────────────────────────────────────────────

  function gainToY(gain: number): number {
    const ratio = (EQ_MAX_GAIN - gain) / (EQ_MAX_GAIN - EQ_MIN_GAIN);
    return pad + ratio * inner;
  }

  function yToGain(y: number): number {
    const clampedY = Math.max(pad, Math.min(plotHeight - pad, y));
    const ratio = (clampedY - pad) / inner;
    return EQ_MAX_GAIN - ratio * (EQ_MAX_GAIN - EQ_MIN_GAIN);
  }

  const points = $derived(
    EQ_FREQUENCIES.map((frequency, index) => {
      const x = (index / (EQ_FREQUENCIES.length - 1)) * 100;
      const y = gainToY(bands[index]);
      return { frequency, label: frequencyLabels[index], gain: bands[index], x, y };
    })
  );

  const linePoints = $derived(points.map((p) => `${p.x},${p.y}`).join(' '));

  function updateBand(index: number, value: number) {
    const next = [...bands] as EqBandValues;
    next[index] = Math.round(Math.max(EQ_MIN_GAIN, Math.min(EQ_MAX_GAIN, value)));
    bands = normalizeEqBands(next) as EqBandValues;
    onchange?.(bands);
  }

  function updateBandFromPointer(index: number, clientY: number) {
    if (!graphEl) return;
    const rect = graphEl.getBoundingClientRect();
    const nextY = ((clientY - rect.top) / rect.height) * plotHeight;
    updateBand(index, yToGain(nextY));
  }

  function startDrag(index: number, event: PointerEvent) {
    if (!enabled) return;
    activeDragIndex = index;
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    updateBandFromPointer(index, event.clientY);
  }

  function onPointerMove(event: PointerEvent) {
    if (!enabled || activeDragIndex === null) return;
    updateBandFromPointer(activeDragIndex, event.clientY);
  }

  function stopDrag() {
    activeDragIndex = null;
  }

  // ── Knob helpers ─────────────────────────────────────────────────────────────

  // Knob arc: -135° to +135° from 12-o'clock, total 270°
  const KNOB_START = -135;
  const KNOB_RANGE = 270;
  const R = 16; // arc radius
  const CX = 20;
  const CY = 20;

  function gainToAngle(gain: number): number {
    const ratio = (gain - EQ_MIN_GAIN) / (EQ_MAX_GAIN - EQ_MIN_GAIN);
    return KNOB_START + ratio * KNOB_RANGE;
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

  function knobFillPath(gain: number): string {
    const angle = gainToAngle(gain);
    // Fill from center (0dB = 0°) outward, so it shows boost/cut direction
    const zeroAngle = gainToAngle(0);
    const start = Math.min(zeroAngle, angle);
    const end = Math.max(zeroAngle, angle);
    if (Math.abs(end - start) < 1) return '';
    return arcPath(start, end, R);
  }

  function startKnobDrag(index: number, event: PointerEvent) {
    if (!enabled) return;
    activeKnobIndex = index;
    knobDragStartY = event.clientY;
    knobDragStartGain = bands[index];
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  }

  function onKnobPointerMove(event: PointerEvent) {
    if (!enabled || activeKnobIndex === null) return;
    const dy = knobDragStartY - event.clientY; // drag up = increase
    const delta = dy * (EQ_MAX_GAIN - EQ_MIN_GAIN) / 120;
    updateBand(activeKnobIndex, knobDragStartGain + delta);
  }

  function stopKnobDrag() {
    activeKnobIndex = null;
  }
</script>

<div class="rounded-[28px] border border-border/60 bg-card shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
  <div class="flex gap-4 p-4">
    <div class="flex w-11 shrink-0 flex-col justify-between py-5 text-xs font-semibold text-muted-foreground">
      <span>+12dB</span>
      <span>-12dB</span>
    </div>
    <div class="min-w-0 flex-1">

      <!-- Graph -->
      <div
        class="relative h-48 w-full touch-none overflow-hidden rounded-t-[24px] border border-b-0 border-border/40 bg-card px-8 pt-5 pb-2 {enabled ? '' : 'opacity-50'}"
        role="group"
        aria-label="Equalizer graph"
      >
        <div class="pointer-events-none absolute inset-0" style="background: radial-gradient(circle at top, color-mix(in srgb, var(--foreground) 5%, transparent) 0%, transparent 55%), linear-gradient(180deg, color-mix(in srgb, var(--foreground) 4%, transparent) 0%, rgba(0,0,0,0.14) 100%);"></div>
        <div
          bind:this={graphEl}
          class="absolute inset-x-8 top-5 bottom-2"
          role="presentation"
          onpointermove={onPointerMove}
          onpointerup={stopDrag}
          onpointercancel={stopDrag}
        >
          <div class="pointer-events-none absolute left-0 right-0 top-1/2 h-px -translate-y-1/2 bg-primary/25"></div>

          {#each points as point (point.frequency)}
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

          {#each points as point, index (point.frequency)}
            <button
              type="button"
              class="absolute flex h-8 w-8 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full transition disabled:cursor-not-allowed"
              style={`left: ${point.x}%; top: ${(point.y / plotHeight) * 100}%;`}
              onpointerdown={(event) => startDrag(index, event)}
              onpointerup={stopDrag}
              disabled={!enabled}
              aria-label={`${labels[index]} EQ band`}
            >
              <span class="flex h-3.5 w-3.5 rounded-full border border-primary/70 bg-primary shadow-[0_0_0_5px_color-mix(in_srgb,var(--primary)_18%,transparent)]"></span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Knobs -->
      <div class="grid grid-cols-9 rounded-b-[24px] border border-t-0 border-border/40 bg-card/60 px-8 py-4 {enabled ? '' : 'opacity-50'}">
        {#each points as point, index (point.frequency)}
          {@const fillPath = knobFillPath(point.gain)}
          <div class="flex flex-col items-center gap-1">
            <p class="text-[11px] font-medium text-muted-foreground">{point.label}</p>
            <p class="text-[10px] text-muted-foreground/50">{frequencyLabels[index]}</p>
            <button
              type="button"
              class="touch-none cursor-ns-resize disabled:cursor-not-allowed"
              onpointerdown={(e) => startKnobDrag(index, e)}
              disabled={!enabled}
              aria-label={`${labels[index]} ${point.gain > 0 ? '+' : ''}${point.gain.toFixed(0)} dB`}
            >
              <svg width="40" height="40" viewBox="0 0 40 40" aria-hidden="true">
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
                {#each [gainToAngle(point.gain)] as angle}
                  {@const dot = polarToXY(angle, R)}
                  <circle cx={dot.x} cy={dot.y} r="2.5" fill="var(--primary)" />
                {/each}
              </svg>
            </button>
            <p class="text-[11px] font-semibold tabular-nums text-primary">{point.gain > 0 ? '+' : ''}{point.gain.toFixed(0)} dB</p>
          </div>
        {/each}
      </div>

    </div>
  </div>
</div>

<svelte:window
  onpointermove={onKnobPointerMove}
  onpointerup={() => { stopDrag(); stopKnobDrag(); }}
  onpointercancel={() => { stopDrag(); stopKnobDrag(); }}
/>
