<script lang="ts">
  import ServiceStatusSection from '../ServiceStatusSection.svelte';
  import ServersSection from '../ServersSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="space-y-8">
  <div class="page-section">
    <ServiceStatusSection refreshKey={settings.statsRefreshKey} />
  </div>
  <div class="page-section">
    <ServersSection
      bind:profiles={settings.profiles}
      onHealthChange={settings.refreshStats}
    />
  </div>
</div>
