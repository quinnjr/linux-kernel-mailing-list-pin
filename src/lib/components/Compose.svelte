<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../api";
  import { store } from "../store.svelte";
  import Subject from "./Subject.svelte";

  let to = $state("");
  let cc = $state("");
  let subject = $state("");
  let body = $state("");
  let sending = $state(false);
  let textarea = $state<HTMLTextAreaElement | null>(null);
  let caret = $state({ line: 1, col: 1 });

  onMount(async () => {
    to = await api.defaultRecipient();
  });

  const canSend = $derived(!sending && to.trim() !== "" && subject.trim() !== "" && body.trim() !== "");
  const longLines = $derived(body.split("\n").filter((l) => l.length > 72).length);
  const lineCount = $derived(body === "" ? 0 : body.split("\n").length);

  function updateCaret() {
    if (!textarea) return;
    const before = body.slice(0, textarea.selectionStart);
    const lines = before.split("\n");
    caret = { line: lines.length, col: lines[lines.length - 1].length + 1 };
  }

  async function send() {
    if (!canSend) return;
    sending = true;
    try {
      const t = await api.sendEmail({ to, cc, subject, body });
      store.notify("ok", "Sent. Watching lore.kernel.org for replies.");
      subject = "";
      body = "";
      cc = "";
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
    <div class="grid grid-cols-[6ch_1fr] items-center gap-x-3 gap-y-1.5 font-mono text-[13px]">
      <label class="text-ink-400" for="to">To:</label>
      <input id="to" class="field" bind:value={to} spellcheck="false" />
      <label class="text-ink-400" for="cc">Cc:</label>
      <input id="cc" class="field" bind:value={cc} placeholder="maintainers from get_maintainer.pl, other lists" spellcheck="false" />
      <label class="text-ink-400" for="subject">Subject:</label>
      <input id="subject" class="field" bind:value={subject} placeholder="[RFC] subsystem: what changes and why" />
      {#if subject.trim()}
        <span></span>
        <div class="min-w-0 text-[12px]"><Subject {subject} /></div>
      {/if}
    </div>

    <!-- 72-column guide: the list's line-length convention, drawn as a rule -->
    <div class="relative mt-3 min-h-0 flex-1">
      <textarea
        bind:this={textarea}
        class="field h-full resize-none leading-[1.55]"
        bind:value={body}
        oninput={updateCaret}
        onclick={updateCaret}
        onkeyup={updateCaret}
        placeholder="Plain text. Wrap at 72 columns; the list drops HTML."
        spellcheck="true"
        wrap="off"
      ></textarea>
      <div
        class="pointer-events-none absolute inset-y-0 border-l border-dashed border-ink-600"
        style="left: calc(0.625rem + 1px + 72ch)"
        aria-hidden="true"
      ></div>
    </div>

    <div class="mt-2 flex items-center gap-4 font-mono text-[11px] text-ink-400">
      <span>L{caret.line} C{caret.col}</span>
      <span>{lineCount} line{lineCount === 1 ? "" : "s"}</span>
      {#if longLines}
        <span class="text-del">{longLines} line{longLines === 1 ? "" : "s"} past column 72</span>
      {:else if body}
        <span class="text-add">all lines within 72 columns</span>
      {/if}
      <span class="ml-auto">text/plain · generated Message-ID · tracked on lore</span>
    </div>
  </div>
</div>
