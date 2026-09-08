import type { Reply } from "./api";

export interface TreeRow {
  reply: Reply;
  depth: number;
  /** Connector prefix for the row, in the style of mutt's thread index. */
  prefix: string;
}

/**
 * Arrange replies into a thread tree keyed on In-Reply-To. Replies whose
 * parent is unknown (fetched out of band, or replying to a message that
 * never reached the list) hang directly off the root.
 */
export function threadTree(rootId: string, replies: Reply[]): TreeRow[] {
  const byId = new Map(replies.map((r) => [r.message_id, r]));
  const children = new Map<string, Reply[]>();
  for (const r of replies) {
    const parent = r.in_reply_to && byId.has(r.in_reply_to) ? r.in_reply_to : rootId;
    const list = children.get(parent) ?? [];
    list.push(r);
    children.set(parent, list);
  }
  for (const list of children.values()) list.sort((a, b) => a.date.localeCompare(b.date));

  const rows: TreeRow[] = [];
  const walk = (id: string, depth: number, trunk: string) => {
    const kids = children.get(id) ?? [];
    kids.forEach((k, i) => {
      const last = i === kids.length - 1;
      const branch = depth === 0 ? "" : trunk + (last ? "└─" : "├─");
      rows.push({ reply: k, depth, prefix: branch });
      walk(k.message_id, depth + 1, depth === 0 ? "" : trunk + (last ? "  " : "│ "));
    });
  };
  walk(rootId, 0, "");
  return rows;
}
