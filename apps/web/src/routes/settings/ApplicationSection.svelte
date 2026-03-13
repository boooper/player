<script lang="ts">
  import { Power } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { setAutostart } from '$lib/api';

  let {
    autostartEnabled = $bindable(false),
  }: {
    autostartEnabled: boolean;
  } = $props();

  let autostartLoading = $state(false);

  async function toggleAutostart() {
    autostartLoading = true;
    try {
      const next = !autostartEnabled;
      await setAutostart(next);
      autostartEnabled = next;
    } catch {
      toast.error('Failed to update launch at login');
    } finally {
      autostartLoading = false;
    }
  }
</script>

<section class="rounded-xl border border-border/70 bg-card">
  <div class="border-b border-border/60 px-5 py-4">
    <div class="flex items-center gap-2">
      <Power class="size-4 text-muted-foreground" />
      <h2 class="font-semibold">Application</h2>
    </div>
    <p class="mt-0.5 text-xs text-muted-foreground">System-level application behaviour.</p>
  </div>
  <div class="px-5 py-5">
    <label class="flex cursor-pointer items-center gap-3" for="autostart-toggle">
      <input
        id="autostart-toggle"
        type="checkbox"
        class="sr-only"
        checked={autostartEnabled}
        onchange={toggleAutostart}
        disabled={autostartLoading}
      />
      <div
        aria-hidden="true"
        class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors {autostartEnabled ? 'bg-primary' : 'bg-input'}"
      >
        <span class="pointer-events-none inline-block h-4 w-4 translate-x-0 rounded-full bg-background shadow ring-0 transition-transform {autostartEnabled ? 'translate-x-4' : ''}"></span>
      </div>
      <div>
        <p class="text-sm font-medium">Launch at login</p>
        <p class="text-xs text-muted-foreground">Automatically start Player when you log in.</p>
      </div>
    </label>
  </div>
</section>
