<script lang="ts">
  import IntegrationsSection from '../IntegrationsSection.svelte';
  import { useSettingsContext } from '../context.svelte.js';

  const settings = useSettingsContext();
</script>

<div class="space-y-8">
  <div class="page-section">
    <IntegrationsSection
      isMobile={settings.isMobile}
      bind:lastFmApiKey={settings.lastFmApiKey}
      bind:lastFmSharedSecret={settings.lastFmSharedSecret}
      bind:lastFmSharedSecretConfigured={settings.lastFmSharedSecretConfigured}
      bind:lfmConnected={settings.lfmConnected}
      bind:lfmUsername={settings.lfmUsername}
      bind:listenBrainzUsername={settings.listenBrainzUsername}
      bind:listenBrainzToken={settings.listenBrainzToken}
      bind:listenBrainzTokenConfigured={settings.listenBrainzTokenConfigured}
      bind:metadataProvider={settings.metadataProvider}
      bind:recommendationProvider={settings.recommendationProvider}
      onHealthChange={settings.refreshStats}
    />
  </div>
</div>
