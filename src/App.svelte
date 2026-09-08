<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { store } from "./lib/store.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Threads from "./lib/components/Threads.svelte";
  import Compose from "./lib/components/Compose.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import Toast from "./lib/components/Toast.svelte";

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
</script>

<div class="flex h-full">
  <Sidebar />
  <main class="flex min-w-0 flex-1 flex-col">
    {#if store.view === "threads"}
      <Threads />
    {:else if store.view === "compose"}
      <Compose />
    {:else}
      <Settings />
    {/if}
  </main>
  <Toast />
</div>
