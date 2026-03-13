<script lang="ts">
  import { Trash2, Loader2 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { clearDatabase } from '$lib/api';

  let {
    onCleared,
  }: {
    onCleared: () => void;
  } = $props();

  let clearDbConfirming = $state(false);
  let clearDbLoading = $state(false);
  let clearDbTimer: ReturnType<typeof setTimeout> | null = null;

  function requestClearDb() {
    clearDbConfirming = true;
    clearDbTimer = setTimeout(() => { clearDbConfirming = false; }, 4000);
  }

  async function confirmClearDb() {
    if (clearDbTimer) clearTimeout(clearDbTimer);
    clearDbConfirming = false;
    clearDbLoading = true;
    try {
      await clearDatabase();
      toast.success('Database cleared');
      onCleared();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to clear database');
    } finally {
      clearDbLoading = false;
    }
  }
</script>

<section class="rounded-xl border border-rose-500/30 bg-card">
  <div class="border-b border-rose-500/20 px-5 py-4">
    <div class="flex items-center gap-2">
      <Trash2 class="size-4 text-rose-400" />
      <h2 class="font-semibold text-rose-400">Danger Zone</h2>
    </div>
    <p class="mt-0.5 text-xs text-muted-foreground">Irreversible actions. Proceed with caution.</p>
  </div>
  <div class="flex items-center justify-between px-5 py-4">
    <div>
      <p class="text-sm font-medium">Clear all data</p>
      <p class="text-xs text-muted-foreground">Deletes all settings, servers, and saved artists from the local database.</p>
    </div>
    {#if clearDbConfirming}
      <button
        class="flex items-center gap-2 rounded-lg bg-rose-500 px-4 py-2 text-sm font-semibold text-white transition hover:bg-rose-600 disabled:opacity-60"
        onclick={confirmClearDb}
        disabled={clearDbLoading}
      >
        <Trash2 class="size-4" />
        Confirm — this cannot be undone
      </button>
    {:else}
      <button
        class="flex items-center gap-2 rounded-lg border border-rose-500/50 px-4 py-2 text-sm font-medium text-rose-400 transition hover:bg-rose-500/10 disabled:opacity-60"
        onclick={requestClearDb}
        disabled={clearDbLoading}
      >
        {#if clearDbLoading}
          <Loader2 class="size-4 animate-spin" />
          Clearing…
        {:else}
          <Trash2 class="size-4" />
          Clear Database
        {/if}
      </button>
    {/if}
  </div>
</section>
