<script lang="ts">
  import StatsSection from '../StatsSection.svelte';
  import ServersSection from '../ServersSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="space-y-8">
  <StatsSection refreshKey={settings.statsRefreshKey} />
  <ServersSection
    bind:profiles={settings.profiles}
    onHealthChange={settings.refreshStats}
  />
</div>
