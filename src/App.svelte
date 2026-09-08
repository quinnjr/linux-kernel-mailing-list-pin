<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { store } from "./lib/store.svelte";
  import Rail from "./lib/components/Rail.svelte";
  import StatusLine from "./lib/components/StatusLine.svelte";
  import Threads from "./lib/components/Threads.svelte";
  import Compose from "./lib/components/Compose.svelte";
  import Settings from "./lib/components/Settings.svelte";

  onMount(() => {
    store.loadThreads();
    const unlisten = [
      api.onThreadsUpdated(() => store.loadThreads()),
      api.onNewReplies((n) => store.notify("ok", `${n} new repl${n === 1 ? "y" : "ies"} arrived`)),
    ];
    api.getSettings().then((s) => {
      if (!s.email || !s.smtp_host) store.view = "settings";
    });
    return () => unlisten.forEach((p) => p.then((f) => f()));
  });

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT")) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    // Compare case-insensitively so Caps Lock does not disable the keys;
    // G is the one shifted binding.
    const key = e.key.toLowerCase();
    if (key === "g" && e.shiftKey) store.refreshAll();
    else if (e.shiftKey) return;
    else if (key === "m") store.view = "compose";
    else if (key === "i") store.view = "threads";
    else if (key === "s") store.view = "settings";
    else if (store.view === "threads" && (key === "j" || key === "k")) {
      e.preventDefault();
      store.move(key === "j" ? 1 : -1);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="grid h-full grid-cols-[minmax(0,1fr)] grid-rows-[1fr_auto] overflow-hidden bg-ink-900">
  <div class="flex min-h-0 min-w-0">
    <Rail />
    <main class="flex min-w-0 flex-1 flex-col">
      {#if store.view === "threads"}
        <Threads />
      {:else if store.view === "compose"}
        <Compose />
      {:else}
        <Settings />
      {/if}
    </main>
  </div>
  <StatusLine />
</div>
