<script lang="ts">
  import ServiceStatusSection from '../ServiceStatusSection.svelte';
  import ServersSection from '../ServersSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="space-y-6">
  <ServiceStatusSection refreshKey={settings.statsRefreshKey} />
  <ServersSection
    bind:profiles={settings.profiles}
    onHealthChange={settings.refreshStats}
  />
</div>
