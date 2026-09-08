<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, type Settings } from "../api";
  import { store } from "../store.svelte";

  let s = $state<Settings>({
    display_name: "",
    email: "",
    smtp_host: "",
    smtp_port: 587,
    smtp_user: "",
    smtp_security: "starttls",
    poll_minutes: 15,
    has_password: false,
  });
  let password = $state("");
  let saving = $state(false);

  onMount(async () => {
    s = await api.getSettings();
  });

  async function save() {
    saving = true;
    try {
      s = await api.saveSettings(s, password);
      password = "";
      store.notify("ok", "Settings saved");
    } catch (e) {
      store.notify("err", errorText(e));
    } finally {
      saving = false;
    }
  }

  function onSecurity() {
    if (s.smtp_security === "tls" && s.smtp_port === 587) s.smtp_port = 465;
    if (s.smtp_security === "starttls" && s.smtp_port === 465) s.smtp_port = 587;
  }
</script>

<div class="flex h-full flex-col">
  <header class="flex items-center justify-between border-b border-zinc-800 px-6 py-3">
    <h1 class="text-sm font-semibold">Settings</h1>
    <button
      class="rounded-md bg-amber-500 px-3 py-1.5 text-sm font-medium text-zinc-950 hover:bg-amber-400 disabled:opacity-40"
      onclick={save}
      disabled={saving}
    >
      {saving ? "Saving…" : "Save"}
    </button>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
    <div class="max-w-xl space-y-6">
      <fieldset class="space-y-3">
        <legend class="mb-1 text-xs font-semibold uppercase tracking-wider text-zinc-500">Identity</legend>
        <label class="row"><span>Name</span><input class="field" bind:value={s.display_name} placeholder="Jane Hacker" /></label>
        <label class="row"><span>Email</span><input class="field" bind:value={s.email} placeholder="jane@example.org" spellcheck="false" /></label>
      </fieldset>

      <fieldset class="space-y-3">
        <legend class="mb-1 text-xs font-semibold uppercase tracking-wider text-zinc-500">Outgoing SMTP</legend>
        <label class="row"><span>Host</span><input class="field" bind:value={s.smtp_host} placeholder="smtp.example.org" spellcheck="false" /></label>
        <label class="row">
          <span>Security</span>
          <select class="field" bind:value={s.smtp_security} onchange={onSecurity}>
            <option value="starttls">STARTTLS (usually port 587)</option>
            <option value="tls">Implicit TLS (usually port 465)</option>
            <option value="none">None (local relay only)</option>
          </select>
        </label>
        <label class="row"><span>Port</span><input class="field" type="number" bind:value={s.smtp_port} min="1" max="65535" /></label>
        <label class="row"><span>Username</span><input class="field" bind:value={s.smtp_user} spellcheck="false" autocomplete="off" /></label>
        <label class="row">
          <span>Password</span>
          <input
            class="field"
            type="password"
            bind:value={password}
            autocomplete="off"
            placeholder={s.has_password ? "•••••••• (saved in system keyring)" : "stored in your system keyring"}
          />
        </label>
      </fieldset>

      <fieldset class="space-y-3">
        <legend class="mb-1 text-xs font-semibold uppercase tracking-wider text-zinc-500">Reply tracking</legend>
        <label class="row">
          <span>Poll every</span>
          <div class="flex items-center gap-2">
            <input class="field w-24" type="number" bind:value={s.poll_minutes} min="1" />
            <span class="text-sm text-zinc-500">minutes</span>
          </div>
        </label>
        <p class="text-xs text-zinc-500">
          Replies are read from the public lore.kernel.org archive, so no inbox credentials are needed.
          New messages usually appear on lore within a few minutes of hitting the list.
        </p>
      </fieldset>
    </div>
  </div>
</div>

