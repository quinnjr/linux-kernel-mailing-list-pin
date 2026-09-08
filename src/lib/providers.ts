import type { Settings } from "./api";

export interface Provider {
  id: string;
  label: string;
  host: string;
  port: number;
  security: Settings["smtp_security"];
  /** Username is the full email address. */
  userIsEmail: boolean;
  /** Where to get a password, in the provider's own terms. */
  passwordHelp: string;
  passwordUrl?: string;
  notes: string[];
}

export const providers: Provider[] = [
  {
    id: "gmail",
    label: "Gmail / Google Workspace",
    host: "smtp.gmail.com",
    port: 587,
    security: "starttls",
    userIsEmail: true,
    passwordHelp:
      "Use an App Password, not your Google password. Google only issues one once 2-Step Verification is on.",
    passwordUrl: "https://myaccount.google.com/apppasswords",
    notes: [
      "Google replaces the From address with the account address unless it is a verified “Send mail as” alias in Gmail settings.",
      "Gmail sends plain text unchanged; it does not wrap or reflow lines.",
      "About 500 outgoing recipients a day on personal accounts, 2,000 on Workspace.",
    ],
  },
  {
    id: "fastmail",
    label: "Fastmail",
    host: "smtp.fastmail.com",
    port: 465,
    security: "tls",
    userIsEmail: true,
    passwordHelp: "Use an app password created under Settings → Privacy & Security → Integrations.",
    passwordUrl: "https://app.fastmail.com/settings/security/apps",
    notes: [],
  },
  {
    id: "custom",
    label: "Other server",
    host: "",
    port: 587,
    security: "starttls",
    userIsEmail: false,
    passwordHelp: "",
    notes: [],
  },
];

export function detectProvider(s: Settings): Provider {
  return providers.find((p) => p.host && p.host === s.smtp_host) ?? providers[providers.length - 1];
}

export function guessProviderFromEmail(email: string): Provider | null {
  const domain = email.split("@")[1]?.toLowerCase();
  if (!domain) return null;
  if (domain === "gmail.com" || domain === "googlemail.com") return providers[0];
  if (domain === "fastmail.com" || domain === "fastmail.fm") return providers[1];
  return null;
}
