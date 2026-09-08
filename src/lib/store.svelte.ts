import { api, errorText, type Draft, type ThreadSummary } from "./api";

export type View = "threads" | "compose" | "settings";

class AppStore {
  view = $state<View>("threads");
  threads = $state<ThreadSummary[]>([]);
  selectedId = $state<number | null>(null);
  refreshing = $state(false);
  /** The unsent message lives here so switching views never discards it. */
  draft = $state<Draft>({ to: "", cc: "", subject: "", body: "" });
  status = $state<string>("");
  statusKind = $state<"ok" | "err" | "">("");
  private statusTimer: ReturnType<typeof setTimeout> | null = null;

  get unreadTotal() {
    return this.threads.reduce((n, t) => n + t.unread_count, 0);
  }

  get lastChecked(): string | null {
    let latest: string | null = null;
    for (const t of this.threads) {
      if (t.last_checked_at && (!latest || t.last_checked_at > latest)) latest = t.last_checked_at;
    }
    return latest;
  }

  async loadThreads() {
    try {
      this.threads = await api.listThreads();
    } catch (e) {
      this.notify("err", errorText(e));
    }
  }

  async refreshAll() {
    if (this.refreshing) return;
    this.refreshing = true;
    try {
      const reports = await api.refreshAll();
      const fresh = reports.reduce((n, r) => n + r.new_replies, 0);
      const failed = reports.filter((r) => r.error);
      const pending = reports.filter((r) => !r.on_lore && !r.error).length;
      const parts = [fresh ? `${fresh} new repl${fresh === 1 ? "y" : "ies"}` : "no new replies"];
      if (pending) parts.push(`${pending} not on lore yet`);
      if (failed.length) {
        this.notify("err", `${failed.length} of ${reports.length} checks failed: ${failed[0].error}`);
      } else {
        this.notify("ok", parts.join(" · "));
      }
    } catch (e) {
      this.notify("err", errorText(e));
    } finally {
      this.refreshing = false;
      await this.loadThreads();
    }
  }

  /** Step the selection through the index, mutt style (j/k). */
  move(delta: 1 | -1) {
    if (this.threads.length === 0) return;
    const i = this.threads.findIndex((t) => t.id === this.selectedId);
    const next = i < 0 ? (delta > 0 ? 0 : this.threads.length - 1) : Math.min(this.threads.length - 1, Math.max(0, i + delta));
    this.selectedId = this.threads[next].id;
    document.getElementById(`thread-${this.selectedId}`)?.scrollIntoView({ block: "nearest" });
  }

  open(id: number) {
    this.selectedId = id;
    this.view = "threads";
  }

  notify(kind: "ok" | "err", text: string) {
    this.status = text;
    this.statusKind = kind;
    if (this.statusTimer) clearTimeout(this.statusTimer);
    this.statusTimer = setTimeout(() => {
      this.status = "";
      this.statusKind = "";
    }, kind === "err" ? 10000 : 5000);
  }
}

export const store = new AppStore();
