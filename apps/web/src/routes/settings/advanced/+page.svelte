<script lang="ts">
  import ApplicationSection from '../ApplicationSection.svelte';
  import DangerZoneSection from '../DangerZoneSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="space-y-8">
  {#if !settings.isMobile}
    <ApplicationSection
      bind:autostartEnabled={settings.autostartEnabled}
      bind:crossfadeSeconds={settings.crossfadeSeconds}
      bind:eqEnabled={settings.eqEnabled}
      bind:eqPreset={settings.eqPreset}
      bind:eqBands={settings.eqBands}
      showPlayback={false}
      showEqualizer={false}
    />
  {/if}
  <DangerZoneSection onCleared={settings.handleCleared} />
</div>
