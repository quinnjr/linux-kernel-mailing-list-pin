<script lang="ts">
  import { store } from "../store.svelte";
  import { relativeTime } from "../format";
  import ThreadView from "./ThreadView.svelte";
</script>

<div class="flex h-full min-h-0">
  <section class="flex w-80 shrink-0 flex-col border-r border-zinc-800">
    <header class="flex items-center justify-between border-b border-zinc-800 px-4 py-3">
      <h1 class="text-sm font-semibold">Sent threads</h1>
      <span class="text-xs text-zinc-500">{store.threads.length}</span>
    </header>
    {#if store.threads.length === 0}
      <div class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center text-sm text-zinc-500">
        <p>Nothing sent yet.</p>
        <button class="text-amber-400 hover:underline" onclick={() => (store.view = "compose")}>
          Compose your first email
        </button>
      </div>
    {:else}
      <ul class="min-h-0 flex-1 overflow-y-auto">
        {#each store.threads as t (t.id)}
          <li>
            <button
              class="block w-full border-b border-zinc-800/70 px-4 py-3 text-left transition hover:bg-zinc-900
                {store.selectedId === t.id ? 'bg-zinc-900' : ''}"
              onclick={() => store.open(t.id)}
            >
              <div class="flex items-start gap-2">
                <div class="min-w-0 flex-1">
                  <div class="truncate text-sm {t.unread_count ? 'font-semibold text-zinc-50' : 'text-zinc-200'}">
                    {t.subject || "(no subject)"}
                  </div>
                  <div class="mt-0.5 flex items-center gap-2 text-[11px] text-zinc-500">
                    <span>{relativeTime(t.last_activity)}</span>
                    <span>·</span>
                    <span>{t.reply_count} repl{t.reply_count === 1 ? "y" : "ies"}</span>
                  </div>
                </div>
                {#if t.unread_count}
                  <span class="mt-1 rounded-full bg-amber-500 px-1.5 text-[10px] font-bold text-zinc-950">
                    {t.unread_count}
                  </span>
                {/if}
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="min-w-0 flex-1">
    {#if store.selectedId !== null}
      {#key store.selectedId}
        <ThreadView id={store.selectedId} />
      {/key}
    {:else}
      <div class="flex h-full items-center justify-center text-sm text-zinc-600">
        Select a thread to see replies
      </div>
    {/if}
  </section>
</div>
