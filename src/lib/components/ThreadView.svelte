<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText, type ThreadDetail } from "../api";
  import { store } from "../store.svelte";
  import { absoluteTime, relativeTime } from "../format";
  import { threadTree } from "../tree";
  import Subject from "./Subject.svelte";
  import Message from "./Message.svelte";

  /** How long a thread must stay on screen before it counts as read, so j/k skimming keeps flags. */
  const READ_DWELL_MS = 1500;

  let { id }: { id: number } = $props();
  let detail = $state<ThreadDetail | null>(null);
  let busy = $state(false);
  let confirmDelete = $state(false);
  /** Replies that were unread at any point while this view was open keep their "N" flag. */
  let flagged = $state<Set<string>>(new Set());
  let dwelled = false;

  const rows = $derived(detail ? threadTree(detail.summary.message_id, detail.replies) : []);

  async function load() {
    try {
      detail = await api.getThread(id);
    } catch (e) {
      store.notify("err", errorText(e));
      return;
    }
    if (!detail) return;
    const fresh = detail.replies.filter((r) => !r.read).map((r) => r.message_id);
    if (fresh.length) {
      flagged = new Set([...flagged, ...fresh]);
      if (dwelled) api.markThreadRead(id);
    }
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
    if (!confirmDelete) {
      confirmDelete = true;
      setTimeout(() => (confirmDelete = false), 4000);
      return;
    }
    await api.deleteThread(id);
    store.selectedId = null;
  }

  onMount(() => {
    const unlisten = api.onThreadsUpdated(load);
    load();
    const dwell = setTimeout(() => {
      dwelled = true;
      if (detail?.replies.some((r) => !r.read)) api.markThreadRead(id);
    }, READ_DWELL_MS);
    return () => {
      clearTimeout(dwell);
      unlisten.then((f) => f());
    };
  });
</script>

{#if detail}
  {@const s = detail.summary}
  <div class="flex h-full flex-col">
    <header class="border-b border-ink-700 px-6 py-4">
      <div class="flex items-start gap-3">
        <h2 class="min-w-0 flex-1 text-[15px] leading-snug">
          <Subject subject={s.subject} strong />
        </h2>
        <div class="flex shrink-0 gap-1.5">
          <button class="btn" onclick={refresh} disabled={busy}>{busy ? "Checking…" : "Check lore"}</button>
          <button class="btn" onclick={() => openUrl(s.lore_url)}>Open on lore ↗</button>
          <button class="btn btn-danger {confirmDelete ? 'border-del text-ink-50' : ''}" onclick={remove}>
            {confirmDelete ? "Click again to stop tracking" : "Stop tracking"}
          </button>
        </div>
      </div>
      {#if s.status === "unconfirmed"}
        <p class="mt-3 border-l-2 border-flag pl-3 font-mono text-[11.5px] leading-relaxed text-ink-200">
          The mail server reported an error during the send, so delivery is unconfirmed. If this thread
          appears on lore it went out; if it never does, stop tracking it and send again.
        </p>
      {/if}
      <dl class="mt-3 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-0.5 font-mono text-[11.5px]">
        <dt class="text-ink-400">To</dt><dd class="truncate text-ink-200">{s.to_addr}</dd>
        {#if s.cc}<dt class="text-ink-400">Cc</dt><dd class="truncate text-ink-200">{s.cc}</dd>{/if}
        <dt class="text-ink-400">Sent</dt><dd class="text-ink-200">{absoluteTime(s.sent_at)}</dd>
        <dt class="text-ink-400">Checked</dt>
        <dd class="text-ink-200">{s.last_checked_at ? relativeTime(s.last_checked_at) : "never"}</dd>
        <dt class="text-ink-400">Message-ID</dt><dd class="truncate text-ink-300">{s.message_id}</dd>
      </dl>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-4">
      <Message from="You" date={s.sent_at} body={detail.body} mine prefix="" depth={0} />
      {#if rows.length === 0}
        <p class="mt-8 text-center font-mono text-[12.5px] text-ink-400">
          No replies on lore yet. Checked automatically every few minutes.
        </p>
      {/if}
      {#each rows as row (row.reply.id)}
        <Message
          from={row.reply.from_name || row.reply.from_addr}
          addr={row.reply.from_name ? row.reply.from_addr : ""}
          date={row.reply.date}
          body={row.reply.body}
          loreUrl={row.reply.lore_url}
          prefix={row.prefix}
          depth={row.depth}
          unread={flagged.has(row.reply.message_id)}
        />
      {/each}
    </div>
  </div>
{:else}
  <div class="flex h-full items-center justify-center font-mono text-[13px] text-ink-400">Loading…</div>
{/if}
