<script lang="ts">
  import { store } from "../store.svelte";
</script>

{#if store.toast}
  <div
    class="pointer-events-none fixed bottom-4 right-4 max-w-md rounded-md border px-3 py-2 text-sm shadow-lg
      {store.toast.kind === 'err'
        ? 'border-red-800 bg-red-950/90 text-red-100'
        : 'border-zinc-700 bg-zinc-900/95 text-zinc-100'}"
  >
    {store.toast.text}
  </div>
{/if}
