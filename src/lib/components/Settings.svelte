<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText, type Settings } from "../api";
  import { store } from "../store.svelte";
  import { detectProvider, guessProviderFromEmail, providers, type Provider } from "../providers";

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
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; text: string } | null>(null);
  let provider = $state<Provider>(providers[providers.length - 1]);

  onMount(async () => {
    s = await api.getSettings();
    provider = detectProvider(s);
  });

  function applyProvider(p: Provider) {
    provider = p;
    if (p.host) {
      s.smtp_host = p.host;
      s.smtp_port = p.port;
      s.smtp_security = p.security;
    }
    if (p.userIsEmail && s.email) s.smtp_user = s.email;
    testResult = null;
  }

  function onEmailChange() {
    if (provider.userIsEmail) s.smtp_user = s.email;
    const g = guessProviderFromEmail(s.email);
    if (g && !s.smtp_host) applyProvider(g);
  }

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

  async function test() {
    testing = true;
    testResult = null;
    try {
      await api.testSmtp(s, password);
      testResult = { ok: true, text: `Connected to ${s.smtp_host}:${s.smtp_port} and signed in.` };
    } catch (e) {
      testResult = { ok: false, text: explain(errorText(e)) };
    } finally {
      testing = false;
    }
  }

  /** Turn the common SMTP failures into the fix, in the provider's own words. */
  function explain(msg: string): string {
    const m = msg.toLowerCase();
    if (provider.id === "gmail") {
      if (m.includes("535") || m.includes("not accepted") || m.includes("badcredentials"))
        return "Google rejected the sign-in. Use a 16-character App Password (2-Step Verification must be on) and your full address as the username.";
      if (m.includes("534"))
        return "Google wants an App Password for this account; the normal account password is not accepted over SMTP.";
    }
    if (m.includes("535")) return "The server rejected the username or password.";
    if (m.includes("tls") || m.includes("certificate"))
      return `TLS failed: ${msg}. Check that Security matches the port (STARTTLS for 587, implicit TLS for 465).`;
    if (m.includes("refused") || m.includes("timed out") || m.includes("dns") || m.includes("resolve"))
      return `Could not reach ${s.smtp_host}:${s.smtp_port}. ${msg}`;
    return msg;
  }

  function onSecurity() {
    if (s.smtp_security === "tls" && s.smtp_port === 587) s.smtp_port = 465;
    if (s.smtp_security === "starttls" && s.smtp_port === 465) s.smtp_port = 587;
  }

  const gmailFromMismatch = $derived(
    provider.id === "gmail" &&
      s.email !== "" &&
      s.smtp_user !== "" &&
      s.email.toLowerCase() !== s.smtp_user.toLowerCase(),
  );
</script>

<div class="flex h-full flex-col">
  <header class="flex items-center justify-between border-b border-ink-700 px-6 py-2.5">
    <h1 class="eyebrow">Settings</h1>
    <div class="flex gap-1.5">
      <button class="btn" onclick={test} disabled={testing || !s.smtp_host}>
        {testing ? "Connecting…" : "Test connection"}
      </button>
      <button class="btn-primary" onclick={save} disabled={saving}>{saving ? "Saving…" : "Save settings"}</button>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
    <div class="max-w-xl space-y-8">
      <fieldset class="space-y-2.5">
        <legend class="eyebrow mb-2">From header</legend>
        <label class="row"><span>Name</span><input class="field" bind:value={s.display_name} placeholder="Jane Hacker" /></label>
        <label class="row">
          <span>Email</span>
          <input class="field" bind:value={s.email} oninput={onEmailChange} placeholder="jane@example.org" spellcheck="false" />
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
        <label class="row"><span>SMTP host</span><input class="field" bind:value={s.smtp_host} placeholder="smtp.example.org" spellcheck="false" /></label>
        <label class="row">
          <span>Security</span>
          <select class="field" bind:value={s.smtp_security} onchange={onSecurity}>
            <option value="starttls">STARTTLS, port 587</option>
            <option value="tls">Implicit TLS, port 465</option>
            <option value="none">None, local relay only</option>
          </select>
        </label>
        <label class="row"><span>Port</span><input class="field w-28" type="number" bind:value={s.smtp_port} min="1" max="65535" /></label>
        <label class="row">
          <span>Username</span>
          <input
            class="field"
            bind:value={s.smtp_user}
            spellcheck="false"
            autocomplete="off"
            placeholder={provider.userIsEmail ? "your full address" : ""}
          />
        </label>
        <label class="row">
          <span>Password</span>
          <input
            class="field"
            type="password"
            bind:value={password}
            autocomplete="off"
            placeholder={s.has_password
              ? "saved in the system keyring; type to replace"
              : provider.id === "gmail"
                ? "16-character App Password"
                : "kept in the system keyring"}
          />
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
            </p>
          </div>
        {/if}
      </fieldset>

      <fieldset class="space-y-2.5">
        <legend class="eyebrow mb-2">Reply tracking</legend>
        <label class="row">
          <span>Check every</span>
          <div class="flex items-center gap-2">
            <input class="field w-20" type="number" bind:value={s.poll_minutes} min="1" />
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
