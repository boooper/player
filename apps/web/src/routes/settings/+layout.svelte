<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Loader2, Save, Settings, Server, AudioLines, PlugZap, ShieldAlert, BarChart2 } from '@lucide/svelte';
  import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
  import { setSettingsContext } from './context.svelte.js';

  let { children } = $props();

  const settings = setSettingsContext();

  const sections = [
    { value: 'servers', label: 'Servers', href: '/settings/servers', icon: Server },
    { value: 'integrations', label: 'Integrations', href: '/settings/integrations', icon: PlugZap },
    { value: 'sound', label: 'Sound', href: '/settings/sound', icon: AudioLines },
    { value: 'advanced', label: 'Advanced', href: '/settings/advanced', icon: ShieldAlert },
    { value: 'stats', label: 'Stats', href: '/settings/stats', icon: BarChart2 },
  ] as const;

  const currentSection = $derived.by(() => {
    const match = page.url.pathname.match(/^\/settings\/([^/]+)/);
    return match?.[1] ?? 'servers';
  });

  onMount(() => {
    settings.initialize();
  });
</script>

<div class="mx-auto max-w-5xl space-y-6">
  <div class="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
    <div class="flex items-start gap-3">
      <Settings class="mt-0.5 size-7 shrink-0 text-muted-foreground" />
      <div>
        <h1 class="text-2xl font-bold tracking-tight">Settings</h1>
        <p class="text-sm text-muted-foreground">Split by section so server, integration, and sound controls have their own route.</p>
      </div>
    </div>

    <button
      class="flex items-center justify-center gap-2 rounded-lg bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground shadow transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
      onclick={settings.save}
      disabled={settings.saving}
    >
      {#if settings.saving}
        <Loader2 class="size-4 animate-spin" />
        Saving...
      {:else}
        <Save class="size-4" />
        Save Changes
      {/if}
    </button>
  </div>

  <Tabs value={currentSection} class="gap-4">
    <TabsList class="h-auto w-full flex-wrap justify-start gap-2 rounded-2xl bg-secondary/40 p-2">
      {#each sections as section (section.value)}
        {@const Icon = section.icon}
        <TabsTrigger
          value={section.value}
          class="h-10 flex-none gap-2 rounded-xl px-4"
          onclick={() => goto(section.href)}
        >
          <Icon class="size-4" />
          {section.label}
        </TabsTrigger>
      {/each}
    </TabsList>
  </Tabs>

  <div class="min-h-[28rem]">
    {@render children()}
  </div>
</div>
