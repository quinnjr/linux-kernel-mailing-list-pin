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
    if (e.key === "m") store.view = "compose";
    else if (e.key === "i") store.view = "threads";
    else if (e.key === "s") store.view = "settings";
    else if (e.key === "G") store.refreshAll();
    else if (store.view === "threads" && (e.key === "j" || e.key === "k")) {
      e.preventDefault();
      store.move(e.key === "j" ? 1 : -1);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="grid h-full grid-rows-[1fr_auto] bg-ink-900">
  <div class="flex min-h-0">
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
