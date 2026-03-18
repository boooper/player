<script lang="ts">
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator
  } from '$lib/components/ui/dropdown-menu';
  import { Button } from '$lib/components/ui';
  import { Cast } from '@lucide/svelte';
  import type { CastDeviceInfo } from '$lib/servers';

  type Props = {
    castActive: boolean;
    discovering: boolean;
    castDevice: CastDeviceInfo | null;
    castDevices: CastDeviceInfo[];
    disabled?: boolean;
    onDiscover: () => void;
    onStartCast: (device: CastDeviceInfo) => void;
    onStopCast: () => void;
  };

  let {
    castActive,
    discovering,
    castDevice,
    castDevices,
    disabled = false,
    onDiscover,
    onStartCast,
    onStopCast
  }: Props = $props();
</script>

<DropdownMenu onOpenChange={(open) => { if (open && !castActive && !discovering) onDiscover(); }}>
  <DropdownMenuTrigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="ghost"
        size="icon-sm"
        class={`player-transport-button ${castActive ? 'text-primary is-active' : 'text-muted-foreground hover:text-foreground'}`}
        aria-label="Cast"
      >
        <Cast class="size-[18px] {castActive ? 'player-cast-icon' : ''}" />
      </Button>
    {/snippet}
  </DropdownMenuTrigger>

  <DropdownMenuContent side="top" align="end" class="min-w-48">
    {#if castActive && castDevice}
      <div class="px-2 py-1.5">
        <p class="text-[11px] uppercase tracking-wider text-muted-foreground">Casting to</p>
        <p class="text-sm font-semibold">{castDevice.name}</p>
      </div>
      <DropdownMenuSeparator />
      <DropdownMenuItem onclick={onStopCast} class="text-destructive focus:text-destructive">
        Stop casting
      </DropdownMenuItem>
    {:else if discovering}
      <div class="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground">
        <span class="block size-3 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
        Discovering devices…
      </div>
    {:else if castDevices.length === 0}
      <div class="px-3 py-2 text-sm text-muted-foreground">No Cast devices found</div>
      <DropdownMenuSeparator />
      <DropdownMenuItem onclick={onDiscover}>Scan again</DropdownMenuItem>
    {:else}
      <div class="px-2 py-1 text-[11px] uppercase tracking-wider text-muted-foreground">Cast to device</div>
      {#each castDevices as device (device.addr)}
        <DropdownMenuItem onclick={() => onStartCast(device)} disabled={disabled} class="gap-2">
          <Cast class="size-4 shrink-0" />
          {device.name}
        </DropdownMenuItem>
      {/each}
      <DropdownMenuSeparator />
      <DropdownMenuItem onclick={onDiscover}>Scan again</DropdownMenuItem>
    {/if}
  </DropdownMenuContent>
</DropdownMenu>