/** Split "[PATCH v3 2/7] [RFC] mm: do a thing" into its bracketed tags and the rest. */
export interface ParsedSubject {
  tags: string[];
  rest: string;
  /** true when the subject is a reply ("Re:"), which is stripped from `rest`. */
  reply: boolean;
}

export function parseSubject(subject: string): ParsedSubject {
  let s = subject.trim();
  let reply = false;
  const tags: string[] = [];
  for (;;) {
    const re = /^re:\s*/i.exec(s);
    if (re) {
      reply = true;
      s = s.slice(re[0].length);
      continue;
    }
    const tag = /^\[([^\]]{1,40})\]\s*/.exec(s);
    if (tag) {
      tags.push(tag[1].trim());
      s = s.slice(tag[0].length);
      continue;
    }
    break;
  }
  return { tags, rest: s, reply };
}

export function tagClass(tag: string): string {
  const t = tag.toUpperCase();
  if (t.startsWith("RFC")) return "tag tag-rfc";
  if (t.startsWith("PATCH")) return "tag tag-patch";
  return "tag";
}
