<script lang="ts">
  import { parseSubject, tagClass } from "../subject";
  let { subject, strong = false }: { subject: string; strong?: boolean } = $props();
  const p = $derived(parseSubject(subject));
</script>

<span class="flex min-w-0 max-w-full items-baseline gap-1.5">
  {#if p.reply}<span class="font-mono text-[11px] text-ink-400">Re:</span>{/if}
  {#each p.tags as t}<span class={tagClass(t)}>{t}</span>{/each}
  <span class="min-w-0 truncate font-mono text-[13px] {strong ? 'font-semibold text-ink-50' : 'text-ink-100'}">
    {p.rest || "(no subject)"}
  </span>
</span>
