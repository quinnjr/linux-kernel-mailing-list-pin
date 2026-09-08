<script lang="ts">
  import { store } from "../store.svelte";
  import { relativeTime } from "../format";
  import Subject from "./Subject.svelte";
  import ThreadView from "./ThreadView.svelte";

  function stamp(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return "";
    return d.toLocaleDateString(undefined, { month: "short", day: "2-digit" });
  }
</script>

<div class="flex h-full min-h-0">
  <section class="flex w-[26rem] shrink-0 flex-col border-r border-ink-700">
    <header class="flex items-baseline justify-between border-b border-ink-700 px-4 py-2.5">
      <h1 class="eyebrow">Sent threads</h1>
      <span class="font-mono text-[11px] text-ink-400">{store.threads.length}</span>
    </header>
    {#if store.threads.length === 0}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center">
        <p class="font-mono text-[13px] text-ink-300">No threads yet.</p>
        <button class="btn-primary" onclick={() => (store.view = "compose")}>Write to the list</button>
      </div>
    {:else}
      <!-- mutt index columns: flag · date · (replies) · subject -->
      <ul class="min-h-0 flex-1 overflow-y-auto">
        {#each store.threads as t (t.id)}
          <li>
            <button
              id="thread-{t.id}"
              class="grid w-full grid-cols-[1ch_5ch_5ch_minmax(0,1fr)] items-baseline gap-x-2 border-b border-ink-800 px-4 py-2 text-left transition-colors hover:bg-ink-800/60
                {store.selectedId === t.id ? 'bg-ink-800' : ''}"
              onclick={() => store.open(t.id)}
              aria-current={store.selectedId === t.id ? "true" : undefined}
            >
              <span class="font-mono text-[12px] font-semibold text-flag">{t.unread_count ? "N" : " "}</span>
              <span class="font-mono text-[11.5px] text-ink-400">{stamp(t.last_activity)}</span>
              <span class="whitespace-pre font-mono text-[11.5px] {t.reply_count ? 'text-add' : 'text-ink-600'}">
                ({String(t.reply_count).padStart(3, " ")})
              </span>
              <Subject subject={t.subject} strong={t.unread_count > 0} />
              <span class="col-start-4 mt-0.5 truncate font-mono text-[11px] text-ink-400">
                {t.last_checked_at ? `checked ${relativeTime(t.last_checked_at)}` : "not checked yet"}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="min-w-0 flex-1 bg-ink-900">
    {#if store.selectedId !== null}
      {#key store.selectedId}
        <ThreadView id={store.selectedId} />
      {/key}
    {:else}
      <div class="flex h-full items-center justify-center font-mono text-[13px] text-ink-400">
        Select a thread to read its replies
      </div>
    {/if}
  </section>
</div>
