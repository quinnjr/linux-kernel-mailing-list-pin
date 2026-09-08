import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Settings {
  display_name: string;
  email: string;
  smtp_host: string;
  smtp_port: number;
  smtp_user: string;
  smtp_security: "starttls" | "tls" | "none";
  poll_minutes: number;
  has_password: boolean;
}

export interface Draft {
  to: string;
  cc: string;
  subject: string;
  body: string;
}

export interface ThreadSummary {
  id: number;
  message_id: string;
  subject: string;
  to_addr: string;
  cc: string;
  sent_at: string;
  last_checked_at: string | null;
  lore_url: string;
  status: "sent" | "unconfirmed";
  reply_count: number;
  unread_count: number;
  last_activity: string;
}

export interface Reply {
  id: number;
  thread_id: number;
  message_id: string;
  in_reply_to: string | null;
  from_name: string;
  from_addr: string;
  date: string;
  subject: string;
  body: string;
  read: boolean;
  lore_url: string;
}

export interface ThreadDetail {
  summary: ThreadSummary;
  body: string;
  replies: Reply[];
}

export interface RefreshReport {
  thread_id: number;
  new_replies: number;
  on_lore: boolean;
  error: string | null;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings, password?: string) =>
    invoke<Settings>("save_settings", { settings, password: password || null }),
  testSmtp: (settings: Settings, password?: string) =>
    invoke<string>("test_smtp", { settings, password: password || null }),
  forgetPassword: () => invoke<Settings>("forget_password"),
  defaultRecipient: () => invoke<string>("default_recipient"),
  sendEmail: (draft: Draft) => invoke<ThreadSummary>("send_email", { draft }),
  listThreads: () => invoke<ThreadSummary[]>("list_threads"),
  getThread: (id: number) => invoke<ThreadDetail | null>("get_thread", { id }),
  markThreadRead: (id: number) => invoke<void>("mark_thread_read", { id }),
  deleteThread: (id: number) => invoke<void>("delete_thread", { id }),
  refreshThread: (id: number) => invoke<RefreshReport>("refresh_thread", { id }),
  refreshAll: () => invoke<RefreshReport[]>("refresh_all"),
  onThreadsUpdated: (cb: () => void): Promise<UnlistenFn> => listen("threads-updated", cb),
  onNewReplies: (cb: (n: number) => void): Promise<UnlistenFn> =>
    listen<number>("new-replies", (e) => cb(e.payload)),
};

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
