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
    prefix = "",
    depth = 0,
    unread = false,
  }: {
    from: string;
    addr?: string;
    date: string;
    body: string;
    mine?: boolean;
    loreUrl?: string;
    prefix?: string;
    depth?: number;
    unread?: boolean;
  } = $props();

  let collapsed = $state(false);

  /** Quoted lines get the muted treatment mutt gives them. */
  const lines = $derived(body.replace(/\s+$/, "").split("\n"));
</script>

<article
  class="mb-2 grid grid-cols-[auto_minmax(0,1fr)] font-mono"
  style="padding-left: {Math.max(0, depth - 1) * 1.5}ch"
>
  <!-- thread connector, drawn in the gutter like mutt's index -->
  <div class="select-none whitespace-pre pr-2 text-[12.5px] leading-[22px] text-ink-600" aria-hidden="true">
    {prefix}{depth > 0 ? ">" : " "}
  </div>
  <div class="min-w-0 rounded-sm border {mine ? 'border-ink-600 bg-ink-800/50' : 'border-ink-700 bg-ink-950/60'}">
    <header class="flex items-center gap-2 px-3 text-[12px] leading-[22px]">
      <button
        class="w-3 text-ink-400 hover:text-ink-100"
        onclick={() => (collapsed = !collapsed)}
        aria-label={collapsed ? "Expand message" : "Collapse message"}
      >
        {collapsed ? "+" : "−"}
      </button>
      {#if unread}<span class="font-semibold text-flag">N</span>{/if}
      <span class="font-medium text-ink-50">{from}</span>
      {#if addr}<span class="truncate text-ink-400">&lt;{addr}&gt;</span>{/if}
      <span class="ml-auto shrink-0 text-ink-400">{absoluteTime(date)}</span>
      {#if loreUrl}
        <button class="shrink-0 text-ink-400 hover:text-ink-50" onclick={() => openUrl(loreUrl)} aria-label="Open on lore">↗</button>
      {/if}
    </header>
    {#if !collapsed}
      <pre class="overflow-x-auto whitespace-pre-wrap border-t border-ink-700/70 px-3 py-2.5 text-[12.5px] leading-[1.55] text-ink-100">{#each lines as l, i}<span class={l.startsWith(">") ? "text-ink-400" : ""}>{l}</span>{#if i < lines.length - 1}{"\n"}{/if}{/each}</pre>
    {/if}
  </div>
</article>
