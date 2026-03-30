<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Check, Loader2, Settings, Server, AudioLines, PlugZap, ShieldAlert, BarChart2, Heart } from '@lucide/svelte';
  import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
  import { setSettingsContext } from './context.svelte.js';

  let { children } = $props();

  const settings = setSettingsContext();

  const sections = [
    { value: 'servers', label: 'Servers', href: '/settings/servers', icon: Server },
    { value: 'integrations', label: 'Integrations', href: '/settings/integrations', icon: PlugZap },
    { value: 'taste', label: 'Taste', href: '/settings/taste', icon: Heart },
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

    <div class="flex h-8 items-center gap-1.5 text-sm text-muted-foreground transition-opacity duration-300 {settings.saving || settings.savedRecently ? 'opacity-100' : 'opacity-0'}">
      {#if settings.saving}
        <Loader2 class="size-3.5 animate-spin" />
        Saving…
      {:else if settings.savedRecently}
        <Check class="size-3.5 text-emerald-500" />
        <span class="text-emerald-500">Saved</span>
      {/if}
    </div>
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
