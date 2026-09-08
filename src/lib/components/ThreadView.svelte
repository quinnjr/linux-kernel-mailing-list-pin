<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText, type ThreadDetail } from "../api";
  import { store } from "../store.svelte";
  import { absoluteTime, relativeTime } from "../format";
  import Message from "./Message.svelte";

  let { id }: { id: number } = $props();
  let detail = $state<ThreadDetail | null>(null);
  let busy = $state(false);

  async function load() {
    detail = await api.getThread(id);
  }

  async function refresh() {
    busy = true;
    try {
      const r = await api.refreshThread(id);
      store.notify(
        "ok",
        r.on_lore
          ? r.new_replies
            ? `${r.new_replies} new repl${r.new_replies === 1 ? "y" : "ies"}`
            : "No new replies"
          : "lore.kernel.org hasn't indexed this message yet",
      );
      await load();
    } catch (e) {
      store.notify("err", errorText(e));
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!detail) return;
    await api.deleteThread(id);
    store.selectedId = null;
  }

  onMount(() => {
    const unlisten = api.onThreadsUpdated(load);
    load().then(() => {
      if (detail && detail.summary.unread_count > 0) api.markThreadRead(id);
    });
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

{#if detail}
  {@const s = detail.summary}
  <div class="flex h-full flex-col">
    <header class="border-b border-zinc-800 px-6 py-4">
      <div class="flex items-start gap-3">
        <h2 class="min-w-0 flex-1 text-base font-semibold leading-snug">{s.subject || "(no subject)"}</h2>
        <div class="flex shrink-0 gap-1">
          <button
            class="rounded border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800 disabled:opacity-50"
            onclick={refresh}
            disabled={busy}
          >
            {busy ? "Checking…" : "↻ Refresh"}
          </button>
          <button
            class="rounded border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800"
            onclick={() => openUrl(s.lore_url)}
          >
            lore ↗
          </button>
          <button
            class="rounded border border-zinc-800 px-2 py-1 text-xs text-zinc-500 hover:border-red-900 hover:bg-red-950 hover:text-red-200"
            onclick={remove}
          >
            Delete
          </button>
        </div>
      </div>
      <dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs text-zinc-500">
        <dt>To</dt><dd class="truncate text-zinc-300">{s.to_addr}</dd>
        {#if s.cc}<dt>Cc</dt><dd class="truncate text-zinc-300">{s.cc}</dd>{/if}
        <dt>Sent</dt><dd class="text-zinc-300">{absoluteTime(s.sent_at)}</dd>
        <dt>Checked</dt>
        <dd class="text-zinc-300">{s.last_checked_at ? relativeTime(s.last_checked_at) : "never"}</dd>
        <dt>Message-ID</dt><dd class="truncate font-mono text-zinc-400">{s.message_id}</dd>
      </dl>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-4">
      <Message
        from="You"
        addr=""
        date={s.sent_at}
        body={detail.body}
        mine
      />
      {#if detail.replies.length === 0}
        <p class="mt-6 text-center text-sm text-zinc-600">
          No replies yet. The app checks lore.kernel.org in the background.
        </p>
      {/if}
      {#each detail.replies as r (r.id)}
        <Message
          from={r.from_name || r.from_addr}
          addr={r.from_name ? r.from_addr : ""}
          date={r.date}
          body={r.body}
          loreUrl={r.lore_url}
          directReply={r.in_reply_to === s.message_id}
        />
      {/each}
    </div>
  </div>
{:else}
  <div class="flex h-full items-center justify-center text-sm text-zinc-600">Loading…</div>
{/if}
