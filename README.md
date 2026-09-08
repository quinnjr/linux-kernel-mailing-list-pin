# LKML Pin

A small desktop app for sending top-level emails to the Linux kernel mailing
list and keeping track of the replies they get.

- **Send** plain-text emails over SMTP with a locally generated `Message-ID`.
- **Track** replies without any inbox credentials: every message that hits
  `linux-kernel@vger.kernel.org` is archived on [lore.kernel.org](https://lore.kernel.org),
  so the app fetches each sent thread's mbox from there and stores new replies.
- **Poll** lore in the background on a configurable interval and mark unread replies.

Built with Tauri 2, Svelte 5, Tailwind CSS 4, SQLite and lettre.

## Running

Requirements: Rust, Node + pnpm, and the Tauri Linux dependencies
(`webkit2gtk-4.1`, `libsoup3`, `libsecret`).

```sh
pnpm install
pnpm tauri dev          # development
pnpm tauri build        # release bundles under src-tauri/target/release/bundle
```

Tests:

```sh
cd src-tauri
cargo test                       # unit tests
cargo test -- --include-ignored  # also hits lore.kernel.org
pnpm check                       # svelte-check
```

## Configuration

Open **Settings** on first launch and enter your name, address and SMTP
server. The SMTP password is stored in the system keyring (Secret Service on
Linux), never on disk. **Test connection** checks the host, TLS and login
without sending anything.

### Gmail / Google Workspace

Pick the **Gmail / Google Workspace** provider preset (it is selected
automatically for `@gmail.com` addresses). It fills in `smtp.gmail.com`,
STARTTLS on port 587 and sets the username to your address. For the password:

1. Turn on 2-Step Verification for the Google account.
2. Create an App Password at <https://myaccount.google.com/apppasswords>
   (the app shows a link). Paste the 16 characters; the spaces Google shows
   between the groups are ignored. Other passwords are stored exactly as typed.
3. Click **Test connection**, then **Save settings**.

Google rewrites the From header to the account address unless the address is
a verified "Send mail as" alias, and the app warns when they differ. Everything else lives in an SQLite database in the app
data directory (`~/.local/share/dev.quinn.lkml-pin/` on Linux).

## How reply tracking works

After a message is sent, its `Message-ID` is recorded. Refreshing a thread
fetches `https://lore.kernel.org/all/<message-id>/t.mbox.gz`, parses the
mboxrd file and inserts any message in the thread that is not already known.
A 404 simply means lore has not indexed the message yet; try again in a few
minutes. Each reply links back to its lore page.

## Keys

| Key | Action |
| --- | --- |
| `i` / `m` / `s` | Index, Compose, Settings |
| `j` / `k` | Move through the index |
| `G` | Check lore for every thread |
| `Ctrl+Enter` | Send |

An unsent draft is kept while you switch views; it is cleared only after a
successful send. A thread stays unread while you skim past it with `j`/`k`;
it is marked read after it has been on screen for a moment.

If the mail server errors during a send, the thread is kept and marked
"delivery unconfirmed" rather than deleted: SMTP cannot say whether the
message was queued before the failure. If it turns up on lore it went out;
if it never does, stop tracking it and send again.

## Notes

- The icon is Tux by Larry Ewing and The GIMP; see `src-tauri/icons/ATTRIBUTION.md`.

- On NVIDIA + Wayland, WebKitGTK's DMA-BUF renderer crashes on startup. The
  binary sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` unless you override it.
- Emails are sent as `text/plain` only, as the list requires.
