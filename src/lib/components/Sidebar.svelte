<script lang="ts">
  import { store, type View } from "../store.svelte";

  const items: { view: View; label: string; icon: string }[] = [
    { view: "threads", label: "Threads", icon: "▤" },
    { view: "compose", label: "Compose", icon: "✎" },
    { view: "settings", label: "Settings", icon: "⚙" },
  ];
</script>

<nav class="flex w-52 shrink-0 flex-col border-r border-zinc-800 bg-zinc-900/60">
  <div class="px-4 py-4">
    <div class="text-xs font-semibold uppercase tracking-widest text-amber-400">LKML Pin</div>
    <div class="mt-0.5 text-[11px] text-zinc-500">linux-kernel@vger</div>
  </div>
  <ul class="flex flex-col gap-0.5 px-2">
    {#each items as it}
      <li>
        <button
          class="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm transition
            {store.view === it.view
              ? 'bg-zinc-800 text-zinc-50'
              : 'text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-100'}"
          onclick={() => (store.view = it.view)}
        >
          <span class="w-4 text-center text-zinc-500">{it.icon}</span>
          <span class="flex-1">{it.label}</span>
          {#if it.view === "threads" && store.unreadTotal > 0}
            <span class="rounded-full bg-amber-500 px-1.5 text-[10px] font-bold text-zinc-950">
              {store.unreadTotal}
            </span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>
  <div class="mt-auto px-2 pb-3">
    <button
      class="w-full rounded-md border border-zinc-800 px-2.5 py-1.5 text-xs text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-100 disabled:opacity-50"
      onclick={() => store.refreshAll()}
      disabled={store.refreshing}
    >
      {store.refreshing ? "Checking lore…" : "↻ Check for replies"}
    </button>
  </div>
</nav>
