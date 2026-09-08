<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../api";
  import { store } from "../store.svelte";
  import { detectProvider, guessProviderFromEmail, providers, type Provider } from "../providers";

  // The form lives in the store so switching views never loses edits or a typed password.
  const form = store.settingsForm;
  let saving = $state(false);
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; text: string } | null>(null);
  let provider = $state<Provider>(providers[providers.length - 1]);
  /** Email as it was when the username was last auto-filled from it. */
  let usernameFollowsEmail = $state("");

  onMount(async () => {
    if (!form.loaded) {
      try {
        form.s = await api.getSettings();
        form.loaded = true;
      } catch (e) {
        store.notify("err", errorText(e));
        return;
      }
    }
    provider = detectProvider(form.s);
    if (form.s.smtp_user === form.s.email) usernameFollowsEmail = form.s.email;
  });

  /** Number inputs bind null when cleared; send something serde accepts. */
  function normalized() {
    return {
      ...form.s,
      smtp_port: Math.min(65535, Math.max(1, Math.round(Number(form.s.smtp_port) || 587))),
      poll_minutes: Math.min(1440, Math.max(1, Math.round(Number(form.s.poll_minutes) || 15))),
    };
  }

  function onHostChange() {
    provider = detectProvider(form.s);
    testResult = null;
  }

  function applyProvider(p: Provider) {
    provider = p;
    if (p.host) {
      form.s.smtp_host = p.host;
      form.s.smtp_port = p.port;
      form.s.smtp_security = p.security;
    }
    if (p.userIsEmail && form.s.email && !form.s.smtp_user) {
      form.s.smtp_user = form.s.email;
      usernameFollowsEmail = form.s.email;
    }
    testResult = null;
  }

  function onEmailChange() {
    // Only keep the username in step with the email while it was derived from it.
    if (provider.userIsEmail && (form.s.smtp_user === "" || form.s.smtp_user === usernameFollowsEmail)) {
      form.s.smtp_user = form.s.email;
      usernameFollowsEmail = form.s.email;
    }
    const g = guessProviderFromEmail(form.s.email);
    if (g && !form.s.smtp_host) applyProvider(g);
  }

  async function save() {
    saving = true;
    try {
      form.s = await api.saveSettings(normalized(), form.password);
      form.password = "";
      store.notify("ok", "Settings saved");
    } catch (e) {
      store.notify("err", errorText(e));
    } finally {
      saving = false;
    }
  }

  async function forget() {
    try {
      form.s = await api.forgetPassword();
      store.notify("ok", "Password removed from the keyring");
    } catch (e) {
      store.notify("err", errorText(e));
    }
  }

  async function test() {
    testing = true;
    testResult = null;
    try {
      const text = await api.testSmtp(normalized(), form.password);
      testResult = { ok: true, text };
    } catch (e) {
      testResult = { ok: false, text: errorText(e) };
    } finally {
      testing = false;
    }
  }

  function onSecurity() {
    if (form.s.smtp_security === "tls" && form.s.smtp_port === 587) form.s.smtp_port = 465;
    if (form.s.smtp_security === "starttls" && form.s.smtp_port === 465) form.s.smtp_port = 587;
    testResult = null;
  }

  const gmailFromMismatch = $derived(
    provider.id === "gmail" &&
      form.s.email !== "" &&
      form.s.smtp_user !== "" &&
      form.s.email.toLowerCase() !== form.s.smtp_user.toLowerCase(),
  );
</script>

<div class="flex h-full flex-col">
  <header class="flex items-center justify-between border-b border-ink-700 px-6 py-2.5">
    <h1 class="eyebrow">Settings</h1>
    <div class="flex gap-1.5">
      <button class="btn" onclick={test} disabled={testing || !form.s.smtp_host}>
        {testing ? "Connecting…" : "Test connection"}
      </button>
      <button class="btn-primary" onclick={save} disabled={saving}>{saving ? "Saving…" : "Save settings"}</button>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
    <div class="max-w-xl space-y-8">
      <fieldset class="space-y-2.5">
        <legend class="eyebrow mb-2">From header</legend>
        <label class="row"><span>Name</span><input class="field" bind:value={form.s.display_name} placeholder="Jane Hacker" /></label>
        <label class="row">
          <span>Email</span>
          <input class="field" bind:value={form.s.email} oninput={onEmailChange} placeholder="jane@example.org" spellcheck="false" />
        </label>
      </fieldset>

      <fieldset class="space-y-2.5">
        <legend class="eyebrow mb-2">Outgoing mail</legend>
        <div class="row">
          <span>Provider</span>
          <div class="flex gap-1.5" role="radiogroup" aria-label="Mail provider">
            {#each providers as p}
              <button
                class="btn {provider.id === p.id ? 'border-ink-100 bg-ink-700 text-ink-50' : ''}"
                role="radio"
                aria-checked={provider.id === p.id}
                onclick={() => applyProvider(p)}
              >
                {p.label}
              </button>
            {/each}
          </div>
        </div>
        <label class="row"><span>SMTP host</span><input class="field" bind:value={form.s.smtp_host} oninput={onHostChange} placeholder="smtp.example.org" spellcheck="false" /></label>
        <label class="row">
          <span>Security</span>
          <select class="field" bind:value={form.s.smtp_security} onchange={onSecurity}>
            <option value="starttls">STARTTLS, port 587</option>
            <option value="tls">Implicit TLS, port 465</option>
            <option value="none">None, local relay only</option>
          </select>
        </label>
        <label class="row"><span>Port</span><input class="field w-28" type="number" bind:value={form.s.smtp_port} min="1" max="65535" /></label>
        <label class="row">
          <span>Username</span>
          <input
            class="field"
            bind:value={form.s.smtp_user}
            spellcheck="false"
            autocomplete="off"
            placeholder={provider.userIsEmail ? "your full address" : ""}
          />
        </label>
        <label class="row">
          <span>Password</span>
          <div class="flex gap-1.5">
            <input
              class="field"
              type="password"
              bind:value={form.password}
              autocomplete="off"
              placeholder={form.s.has_password
                ? "saved in the system keyring; type to replace"
                : provider.id === "gmail"
                  ? "16-character App Password"
                  : "kept in the system keyring"}
            />
            {#if form.s.has_password}
              <button class="btn shrink-0" onclick={forget}>Forget</button>
            {/if}
          </div>
        </label>
        {#if provider.passwordHelp}
          <div class="row">
            <span></span>
            <p class="font-mono text-[11.5px] leading-relaxed text-ink-300">
              {provider.passwordHelp}
              {#if provider.passwordUrl}
                <button
                  class="ml-1 underline decoration-ink-600 underline-offset-2 hover:text-ink-50"
                  onclick={() => openUrl(provider.passwordUrl!)}
                >
                  Open the App Passwords page ↗
                </button>
              {/if}
            </p>
          </div>
        {/if}
        {#if gmailFromMismatch}
          <div class="row">
            <span></span>
            <p class="border-l-2 border-flag pl-3 font-mono text-[11.5px] leading-relaxed text-ink-200">
              Your From address differs from the Google account. Google will replace it with the account
              address unless it is a verified “Send mail as” alias.
            </p>
          </div>
        {/if}
        {#if provider.notes.length}
          <div class="row">
            <span></span>
            <ul class="space-y-1 font-mono text-[11.5px] leading-relaxed text-ink-400">
              {#each provider.notes as n}<li>· {n}</li>{/each}
            </ul>
          </div>
        {/if}
        {#if testResult}
          <div class="row">
            <span></span>
            <p
              class="border-l-2 pl-3 font-mono text-[12px] leading-relaxed {testResult.ok
                ? 'border-add text-add'
                : 'border-del text-ink-100'}"
            >
              {testResult.text}
              {#if testResult.ok}<span class="text-ink-400"> Not saved yet.</span>{/if}
            </p>
          </div>
        {/if}
      </fieldset>

      <fieldset class="space-y-2.5">
        <legend class="eyebrow mb-2">Reply tracking</legend>
        <label class="row">
          <span>Check every</span>
          <div class="flex items-center gap-2">
            <input class="field w-20" type="number" bind:value={form.s.poll_minutes} min="1" max="1440" />
            <span class="font-mono text-[12px] text-ink-400">minutes</span>
          </div>
        </label>
        <p class="font-mono text-[11.5px] leading-relaxed text-ink-400">
          Replies come from the public lore.kernel.org archive, so no inbox login is needed.
          A message usually shows up on lore a few minutes after the list accepts it.
        </p>
      </fieldset>
    </div>
  </div>
</div>
