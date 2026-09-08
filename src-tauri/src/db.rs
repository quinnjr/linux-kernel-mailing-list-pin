use crate::error::AppResult;
use crate::models::{Reply, Settings, ThreadDetail, ThreadSummary};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    conn: Connection,
}

pub struct NewThread<'a> {
    pub message_id: &'a str,
    pub subject: &'a str,
    pub to_addr: &'a str,
    pub cc: &'a str,
    pub body: &'a str,
    pub sent_at: &'a str,
    pub lore_url: &'a str,
}

pub struct NewReply<'a> {
    pub thread_id: i64,
    pub message_id: &'a str,
    pub in_reply_to: Option<&'a str>,
    pub from_name: &'a str,
    pub from_addr: &'a str,
    pub date: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
    pub lore_url: &'a str,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS threads (
                id INTEGER PRIMARY KEY,
                message_id TEXT NOT NULL UNIQUE,
                subject TEXT NOT NULL,
                to_addr TEXT NOT NULL,
                cc TEXT NOT NULL DEFAULT '',
                body TEXT NOT NULL,
                sent_at TEXT NOT NULL,
                last_checked_at TEXT,
                lore_url TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS replies (
                id INTEGER PRIMARY KEY,
                thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
                message_id TEXT NOT NULL,
                in_reply_to TEXT,
                from_name TEXT NOT NULL DEFAULT '',
                from_addr TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                subject TEXT NOT NULL DEFAULT '',
                body TEXT NOT NULL DEFAULT '',
                read INTEGER NOT NULL DEFAULT 0,
                lore_url TEXT NOT NULL DEFAULT '',
                UNIQUE(thread_id, message_id)
            );
            "#,
        )?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Schema version 1 made `replies.message_id` unique per thread instead
    /// of globally. Replies are a cache of lore, so an old table is simply
    /// rebuilt and refilled on the next check.
    fn migrate(&self) -> AppResult<()> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            let old_global_unique: bool = self
                .conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'replies'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .map(|sql| !sql.contains("UNIQUE(thread_id, message_id)"))
                .unwrap_or(false);
            if old_global_unique {
                self.conn.execute_batch(
                    r#"
                    CREATE TABLE replies_new (
                        id INTEGER PRIMARY KEY,
                        thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
                        message_id TEXT NOT NULL,
                        in_reply_to TEXT,
                        from_name TEXT NOT NULL DEFAULT '',
                        from_addr TEXT NOT NULL DEFAULT '',
                        date TEXT NOT NULL,
                        subject TEXT NOT NULL DEFAULT '',
                        body TEXT NOT NULL DEFAULT '',
                        read INTEGER NOT NULL DEFAULT 0,
                        lore_url TEXT NOT NULL DEFAULT '',
                        UNIQUE(thread_id, message_id)
                    );
                    INSERT INTO replies_new SELECT * FROM replies;
                    DROP TABLE replies;
                    ALTER TABLE replies_new RENAME TO replies;
                    "#,
                )?;
            }
            self.conn.pragma_update(None, "user_version", 1)?;
        }
        Ok(())
    }

    // ----- settings -----

    fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> AppResult<Settings> {
        let d = Settings::defaults();
        let get = |k: &str, fallback: String| -> AppResult<String> {
            Ok(self.get_setting(k)?.unwrap_or(fallback))
        };
        Ok(Settings {
            display_name: get("display_name", d.display_name)?,
            email: get("email", d.email)?,
            smtp_host: get("smtp_host", d.smtp_host)?,
            smtp_port: get("smtp_port", d.smtp_port.to_string())?.parse().unwrap_or(587),
            smtp_user: get("smtp_user", d.smtp_user)?,
            smtp_security: get("smtp_security", d.smtp_security)?,
            poll_minutes: get("poll_minutes", d.poll_minutes.to_string())?.parse().unwrap_or(15),
            has_password: false,
        })
    }

    pub fn save_settings(&self, s: &Settings) -> AppResult<()> {
        self.set_setting("display_name", &s.display_name)?;
        self.set_setting("email", &s.email)?;
        self.set_setting("smtp_host", &s.smtp_host)?;
        self.set_setting("smtp_port", &s.smtp_port.to_string())?;
        self.set_setting("smtp_user", &s.smtp_user)?;
        self.set_setting("smtp_security", &s.smtp_security)?;
        self.set_setting("poll_minutes", &s.poll_minutes.to_string())?;
        Ok(())
    }

    // ----- threads -----

    pub fn insert_thread(&self, t: &NewThread) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO threads(message_id, subject, to_addr, cc, body, sent_at, lore_url)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![t.message_id, t.subject, t.to_addr, t.cc, t.body, t.sent_at, t.lore_url],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    const SUMMARY_SQL: &'static str = r#"
        SELECT t.id, t.message_id, t.subject, t.to_addr, t.cc, t.sent_at, t.last_checked_at, t.lore_url,
               (SELECT COUNT(*) FROM replies r WHERE r.thread_id = t.id),
               (SELECT COUNT(*) FROM replies r WHERE r.thread_id = t.id AND r.read = 0),
               COALESCE((SELECT MAX(r.date) FROM replies r WHERE r.thread_id = t.id), t.sent_at) AS last_activity
        FROM threads t
    "#;

    fn row_to_summary(r: &rusqlite::Row) -> rusqlite::Result<ThreadSummary> {
        Ok(ThreadSummary {
            id: r.get(0)?,
            message_id: r.get(1)?,
            subject: r.get(2)?,
            to_addr: r.get(3)?,
            cc: r.get(4)?,
            sent_at: r.get(5)?,
            last_checked_at: r.get(6)?,
            lore_url: r.get(7)?,
            reply_count: r.get(8)?,
            unread_count: r.get(9)?,
            last_activity: r.get(10)?,
        })
    }

    pub fn list_threads(&self) -> AppResult<Vec<ThreadSummary>> {
        let sql = format!("{} ORDER BY last_activity DESC", Self::SUMMARY_SQL);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_summary)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn thread_ids(&self) -> AppResult<Vec<i64>> {
        let mut stmt = self.conn.prepare("SELECT id FROM threads")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn thread_message_id(&self, id: i64) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT message_id FROM threads WHERE id = ?1", [id], |r| r.get(0))
            .optional()?)
    }

    pub fn get_thread(&self, id: i64) -> AppResult<Option<ThreadDetail>> {
        let sql = format!("{} WHERE t.id = ?1", Self::SUMMARY_SQL);
        let summary = self
            .conn
            .query_row(&sql, [id], Self::row_to_summary)
            .optional()?;
        let Some(summary) = summary else { return Ok(None) };
        let body: String =
            self.conn
                .query_row("SELECT body FROM threads WHERE id = ?1", [id], |r| r.get(0))?;
        let mut stmt = self.conn.prepare(
            "SELECT id, thread_id, message_id, in_reply_to, from_name, from_addr, date, subject, body, read, lore_url
             FROM replies WHERE thread_id = ?1 ORDER BY date ASC",
        )?;
        let replies = stmt
            .query_map([id], |r| {
                Ok(Reply {
                    id: r.get(0)?,
                    thread_id: r.get(1)?,
                    message_id: r.get(2)?,
                    in_reply_to: r.get(3)?,
                    from_name: r.get(4)?,
                    from_addr: r.get(5)?,
                    date: r.get(6)?,
                    subject: r.get(7)?,
                    body: r.get(8)?,
                    read: r.get::<_, i64>(9)? != 0,
                    lore_url: r.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(ThreadDetail { summary, body, replies }))
    }

    pub fn touch_checked(&self, id: i64, when: &str) -> AppResult<()> {
        self.conn.execute(
            "UPDATE threads SET last_checked_at = ?2 WHERE id = ?1",
            params![id, when],
        )?;
        Ok(())
    }

    pub fn delete_thread(&self, id: i64) -> AppResult<()> {
        self.conn.execute("DELETE FROM threads WHERE id = ?1", [id])?;
        Ok(())
    }

    // ----- replies -----

    /// Returns true if the reply was newly inserted.
    pub fn insert_reply(&self, r: &NewReply) -> AppResult<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO replies(thread_id, message_id, in_reply_to, from_name, from_addr, date, subject, body, lore_url)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                r.thread_id, r.message_id, r.in_reply_to, r.from_name, r.from_addr,
                r.date, r.subject, r.body, r.lore_url
            ],
        )?;
        Ok(n > 0)
    }

    pub fn mark_thread_read(&self, thread_id: i64) -> AppResult<()> {
        self.conn.execute(
            "UPDATE replies SET read = 1 WHERE thread_id = ?1",
            [thread_id],
        )?;
        Ok(())
    }
}
