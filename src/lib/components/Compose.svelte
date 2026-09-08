<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../api";
  import { store } from "../store.svelte";

  let to = $state("");
  let cc = $state("");
  let subject = $state("");
  let body = $state("");
  let sending = $state(false);

  onMount(async () => {
    to = await api.defaultRecipient();
  });

  const canSend = $derived(!sending && to.trim() !== "" && subject.trim() !== "" && body.trim() !== "");

  async function send() {
    if (!canSend) return;
    sending = true;
    try {
      const t = await api.sendEmail({ to, cc, subject, body });
      store.notify("ok", "Sent. Tracking replies via lore.kernel.org.");
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
  <header class="flex items-center justify-between border-b border-zinc-800 px-6 py-3">
    <h1 class="text-sm font-semibold">New top-level email</h1>
    <button
      class="rounded-md bg-amber-500 px-3 py-1.5 text-sm font-medium text-zinc-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-40"
      onclick={send}
      disabled={!canSend}
    >
      {sending ? "Sending…" : "Send  ⌃⏎"}
    </button>
  </header>

  <div class="flex min-h-0 flex-1 flex-col gap-2 px-6 py-4">
    <label class="grid grid-cols-[4rem_1fr] items-center gap-2 text-sm">
      <span class="text-zinc-500">To</span>
      <input class="field" bind:value={to} spellcheck="false" />
    </label>
    <label class="grid grid-cols-[4rem_1fr] items-center gap-2 text-sm">
      <span class="text-zinc-500">Cc</span>
      <input class="field" bind:value={cc} placeholder="maintainer@example.org, other-list@vger.kernel.org" spellcheck="false" />
    </label>
    <label class="grid grid-cols-[4rem_1fr] items-center gap-2 text-sm">
      <span class="text-zinc-500">Subject</span>
      <input class="field" bind:value={subject} placeholder="[RFC] …" />
    </label>
    <textarea
      class="field mt-2 min-h-0 flex-1 resize-none font-mono text-[13px] leading-relaxed"
      bind:value={body}
      placeholder="Plain text only. Wrap lines at 72 columns; the list rejects HTML."
      spellcheck="true"
    ></textarea>
    <p class="text-[11px] text-zinc-600">
      Sent as text/plain with a generated Message-ID so replies can be found on lore.kernel.org.
    </p>
  </div>
</div>

