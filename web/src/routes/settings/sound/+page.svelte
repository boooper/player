<script lang="ts">
  import PlaybackSection from '../PlaybackSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="page-section">
  <PlaybackSection
    bind:autostartEnabled={settings.autostartEnabled}
    bind:crossfadeSeconds={settings.crossfadeSeconds}
    bind:eqEnabled={settings.eqEnabled}
    bind:eqPreset={settings.eqPreset}
    bind:eqBands={settings.eqBands}
    title="Sound"
    description={settings.isMobile
      ? 'Playback controls like crossfade and EQ are desktop-only.'
      : 'Crossfade and EQ controls for local playback.'}
    icon="sound"
    showAutostart={false}
    showPlayback={!settings.isMobile}
    showEqualizer={!settings.isMobile}
  />
</div>
