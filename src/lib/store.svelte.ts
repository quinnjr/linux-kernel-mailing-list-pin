import { api, errorText, type ThreadSummary } from "./api";

export type View = "threads" | "compose" | "settings";

class AppStore {
  view = $state<View>("threads");
  threads = $state<ThreadSummary[]>([]);
  selectedId = $state<number | null>(null);
  refreshing = $state(false);
  toast = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  get unreadTotal() {
    return this.threads.reduce((n, t) => n + t.unread_count, 0);
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
      const pending = reports.filter((r) => !r.on_lore).length;
      const parts = [fresh ? `${fresh} new repl${fresh === 1 ? "y" : "ies"}` : "no new replies"];
      if (pending) parts.push(`${pending} not on lore yet`);
      this.notify("ok", parts.join(", "));
    } catch (e) {
      this.notify("err", errorText(e));
    } finally {
      this.refreshing = false;
      await this.loadThreads();
    }
  }

  open(id: number) {
    this.selectedId = id;
    this.view = "threads";
  }

  notify(kind: "ok" | "err", text: string) {
    this.toast = { kind, text };
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = null), kind === "err" ? 8000 : 4000);
  }
}

export const store = new AppStore();
