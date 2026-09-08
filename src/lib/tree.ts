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
    const parentId = firstMsgId(r.in_reply_to);
    const parent = parentId && parentId !== r.message_id && byId.has(parentId) ? parentId : rootId;
    const list = children.get(parent) ?? [];
    list.push(r);
    children.set(parent, list);
  }
  for (const list of children.values()) list.sort((a, b) => a.date.localeCompare(b.date));

  const rows: TreeRow[] = [];
  const seen = new Set<string>();
  const walk = (id: string, depth: number, trunk: string) => {
    const kids = children.get(id) ?? [];
    kids.forEach((k, i) => {
      if (seen.has(k.message_id)) return;
      seen.add(k.message_id);
      const last = i === kids.length - 1;
      const branch = depth === 0 ? "" : trunk + (last ? "└─" : "├─");
      rows.push({ reply: k, depth, prefix: branch });
      walk(k.message_id, depth + 1, depth === 0 ? "" : trunk + (last ? "  " : "│ "));
    });
  };
  walk(rootId, 0, "");
  // Anything unreachable (a cycle among replies) still gets shown, flat at the root.
  for (const r of replies) {
    if (!seen.has(r.message_id)) rows.push({ reply: r, depth: 0, prefix: "" });
  }
  return rows;
}

/** In-Reply-To may carry a comment or several ids; the first <...> is the parent. */
function firstMsgId(v: string | null): string | null {
  if (!v) return null;
  const m = /<[^>]+>/.exec(v);
  return m ? m[0] : v.trim();
}
