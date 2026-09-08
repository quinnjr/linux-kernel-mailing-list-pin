<script lang="ts">
  import { store } from "../store.svelte";
  import { relativeTime } from "../format";

  const replies = $derived(store.threads.reduce((n, t) => n + t.reply_count, 0));
</script>

<!-- mutt's status line: inverse video, one line, the facts you glance at -->
<footer
  class="flex h-6 items-center gap-4 overflow-hidden whitespace-nowrap px-3 font-mono text-[11.5px]
    {store.statusKind === 'err' ? 'bg-del text-ink-950' : 'bg-ink-200 text-ink-900'}"
  aria-live="polite"
>
  <span class="font-semibold">-*-</span>
  <span>{store.threads.length} thread{store.threads.length === 1 ? "" : "s"}</span>
  <span>{replies} repl{replies === 1 ? "y" : "ies"}</span>
  {#if store.unreadTotal}<span class="font-semibold">{store.unreadTotal} unread</span>{/if}
  <span class="text-ink-600">
    checked {store.lastChecked ? relativeTime(store.lastChecked) : "never"}
  </span>
  {#if store.refreshing}<span class="animate-pulse">…checking lore.kernel.org</span>{/if}
  <span class="ml-auto truncate">{store.status}</span>
</footer>
