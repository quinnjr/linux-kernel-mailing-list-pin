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
Linux), never on disk. Everything else lives in an SQLite database in the app
data directory (`~/.local/share/dev.quinn.lkml-pin/` on Linux).

## How reply tracking works

After a message is sent, its `Message-ID` is recorded. Refreshing a thread
fetches `https://lore.kernel.org/all/<message-id>/t.mbox.gz`, parses the
mboxrd file and inserts any message in the thread that is not already known.
A 404 simply means lore has not indexed the message yet; try again in a few
minutes. Each reply links back to its lore page.

## Notes

- On NVIDIA + Wayland, WebKitGTK's DMA-BUF renderer crashes on startup. The
  binary sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` unless you override it.
- Emails are sent as `text/plain` only, as the list requires.
