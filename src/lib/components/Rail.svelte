<script lang="ts">
  import { store, type View } from "../store.svelte";

  const items: { view: View; label: string; key: string }[] = [
    { view: "threads", label: "Index", key: "i" },
    { view: "compose", label: "Compose", key: "m" },
    { view: "settings", label: "Settings", key: "s" },
  ];
</script>

<nav class="flex w-44 shrink-0 flex-col border-r border-ink-700 bg-ink-950">
  <div class="border-b border-ink-700 px-4 pb-3 pt-4">
    <div class="font-cond text-[15px] font-semibold uppercase tracking-[0.2em] text-ink-50">LKML Pin</div>
    <div class="mt-1 font-mono text-[10.5px] text-ink-400">linux-kernel@vger</div>
  </div>
  <ul class="flex flex-col py-2">
    {#each items as it}
      <li>
        <button
          class="flex w-full items-center gap-2 border-l-2 px-4 py-1.5 text-left text-[13px] transition-colors
            {store.view === it.view
              ? 'border-ink-100 bg-ink-800 text-ink-50'
              : 'border-transparent text-ink-300 hover:bg-ink-800/60 hover:text-ink-100'}"
          onclick={() => (store.view = it.view)}
        >
          <span class="flex-1">{it.label}</span>
          {#if it.view === "threads" && store.unreadTotal > 0}
            <span class="font-mono text-[11px] font-semibold text-flag">N {store.unreadTotal}</span>
          {:else}
            <span class="kbd">{it.key}</span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>
  <div class="mt-auto px-4 pb-3 font-mono text-[10.5px] leading-5 text-ink-400">
    <div><span class="kbd">j</span> <span class="kbd">k</span> move</div>
    <div><span class="kbd">^⏎</span> send</div>
  </div>
  <div class="border-t border-ink-700 p-3">
    <button class="btn w-full justify-between" onclick={() => store.refreshAll()} disabled={store.refreshing}>
      <span>{store.refreshing ? "Checking lore…" : "Check lore"}</span>
      <span class="kbd">G</span>
    </button>
  </div>
</nav>
