<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../api";
  import { store } from "../store.svelte";
  import Subject from "./Subject.svelte";

  const draft = store.draft;
  let sending = $state(false);
  let textarea = $state<HTMLTextAreaElement | null>(null);
  let caret = $state({ line: 1, col: 1 });

  onMount(async () => {
    if (!draft.to) draft.to = await api.defaultRecipient();
  });

  const canSend = $derived(
    !sending && draft.to.trim() !== "" && draft.subject.trim() !== "" && draft.body.trim() !== "",
  );
  const longLines = $derived(draft.body.split("\n").filter((l) => l.length > 72).length);
  const lineCount = $derived(draft.body === "" ? 0 : draft.body.split("\n").length);

  function updateCaret() {
    if (!textarea) return;
    const before = draft.body.slice(0, textarea.selectionStart);
    const lines = before.split("\n");
    caret = { line: lines.length, col: lines[lines.length - 1].length + 1 };
  }

  async function send() {
    if (!canSend) return;
    sending = true;
    try {
      const t = await api.sendEmail({ ...draft });
      store.notify("ok", "Sent. Watching lore.kernel.org for replies.");
      draft.subject = "";
      draft.body = "";
      draft.cc = "";
      await store.loadThreads();
      store.open(t.id);
    } catch (e) {
      store.notify("err", errorText(e));
    } finally {
      sending = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="flex h-full flex-col" onkeydown={onKey} role="presentation">
  <header class="flex items-center justify-between border-b border-ink-700 px-6 py-2.5">
    <h1 class="eyebrow">New thread</h1>
    <button class="btn-primary" onclick={send} disabled={!canSend}>
      {sending ? "Sending…" : "Send to list"}
      <span class="kbd !border-ink-400 !text-ink-600">^⏎</span>
    </button>
  </header>

  <div class="flex min-h-0 flex-1 flex-col px-6 py-4">
    <div class="grid grid-cols-[8ch_1fr] items-center gap-x-3 gap-y-1.5 font-mono text-[13px]">
      <label class="text-ink-400" for="to">To:</label>
      <textarea id="to" class="field resize-none" rows="1" bind:value={draft.to} spellcheck="false"></textarea>
      <label class="text-ink-400" for="cc">Cc:</label>
      <textarea
        id="cc"
        class="field resize-y"
        rows="2"
        bind:value={draft.cc}
        placeholder="paste get_maintainer.pl output: one address per line, comments are fine"
        spellcheck="false"
      ></textarea>
      <label class="text-ink-400" for="subject">Subject:</label>
      <input id="subject" class="field" bind:value={draft.subject} placeholder="[RFC] subsystem: what changes and why" />
      {#if draft.subject.trim()}
        <span></span>
        <div class="min-w-0 text-[12px]"><Subject subject={draft.subject} /></div>
      {/if}
    </div>

    <!-- 72-column guide: painted by the textarea itself so it uses the textarea's
         own font metrics and scrolls with the text (background-attachment: local) -->
    <div class="relative mt-3 min-h-0 flex-1">
      <textarea
        bind:this={textarea}
        class="field column-guide h-full resize-none leading-[1.55]"
        bind:value={draft.body}
        oninput={updateCaret}
        onclick={updateCaret}
        onkeyup={updateCaret}
        placeholder="Plain text. Wrap at 72 columns; the list drops HTML."
        spellcheck="true"
        wrap="off"
      ></textarea>
    </div>

    <div class="mt-2 flex items-center gap-4 font-mono text-[11px] text-ink-400">
      <span>L{caret.line} C{caret.col}</span>
      <span>{lineCount} line{lineCount === 1 ? "" : "s"}</span>
      {#if longLines}
        <span class="text-del">{longLines} line{longLines === 1 ? "" : "s"} past column 72</span>
      {:else if draft.body}
        <span class="text-add">all lines within 72 columns</span>
      {/if}
      <span class="ml-auto">draft kept until sent · text/plain · tracked on lore</span>
    </div>
  </div>
</div>
