<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { absoluteTime } from "../format";

  let {
    from,
    addr = "",
    date,
    body,
    mine = false,
    loreUrl = "",
    directReply = false,
  }: {
    from: string;
    addr?: string;
    date: string;
    body: string;
    mine?: boolean;
    loreUrl?: string;
    directReply?: boolean;
  } = $props();

  let collapsed = $state(false);
</script>

<article class="mb-3 rounded-lg border {mine ? 'border-amber-900/60 bg-amber-950/10' : 'border-zinc-800 bg-zinc-900/40'}">
  <header class="flex items-center gap-2 px-4 py-2 text-xs">
    <button class="text-zinc-500 hover:text-zinc-200" onclick={() => (collapsed = !collapsed)}>
      {collapsed ? "▸" : "▾"}
    </button>
    <span class="font-medium text-zinc-100">{from}</span>
    {#if addr}<span class="truncate text-zinc-500">&lt;{addr}&gt;</span>{/if}
    {#if !mine && !directReply}
      <span class="rounded bg-zinc-800 px-1 text-[10px] text-zinc-400" title="Reply to another message in the thread">
        nested
      </span>
    {/if}
    <span class="ml-auto shrink-0 text-zinc-500">{absoluteTime(date)}</span>
    {#if loreUrl}
      <button class="shrink-0 text-zinc-500 hover:text-amber-400" onclick={() => openUrl(loreUrl)}>↗</button>
    {/if}
  </header>
  {#if !collapsed}
    <pre class="overflow-x-auto whitespace-pre-wrap border-t border-zinc-800/60 px-4 py-3 font-mono text-[13px] leading-relaxed text-zinc-200">{body}</pre>
  {/if}
</article>
