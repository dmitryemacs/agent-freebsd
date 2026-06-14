use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct SessionStore {
    conn: Connection,
}

#[allow(dead_code)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub message_count: i64,
}

#[allow(dead_code)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[allow(dead_code)]
impl SessionStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let expanded = if db_path.starts_with("~/") {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(&db_path[2..]).to_string_lossy().to_string()
        } else {
            db_path.to_string()
        };

        let path = Path::new(&expanded);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                title TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session
                ON messages(session_id, id);"
        )?;

        Ok(Self { conn })
    }

    pub fn create_session(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions (id, created_at, updated_at) VALUES (?1, ?2, ?2)",
            rusqlite::params![id, now],
        )?;
        Ok(())
    }

    pub fn save_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![session_id, role, content, now],
        )?;
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    pub fn load_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content, created_at FROM messages WHERE session_id = ?1 ORDER BY id"
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok(StoredMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.created_at, s.updated_at, s.title, COUNT(m.id) as msg_count
             FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
             GROUP BY s.id ORDER BY s.updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionInfo {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                title: row.get(3)?,
                message_count: row.get(4)?,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn last_session(&self) -> Result<Option<String>> {
        let result: Result<Option<String>, _> = self.conn.query_row(
            "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update_title(&self, session_id: &str, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET title = ?1 WHERE id = ?2",
            rusqlite::params![title, session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_session() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_sess_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        let session_id = "test-session-1";
        store.create_session(session_id).unwrap();

        store.save_message(session_id, "user", "hello").unwrap();
        store.save_message(session_id, "assistant", "hi there").unwrap();

        let msgs = store.load_messages(session_id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "hi there");

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session_id);

        let last = store.last_session().unwrap();
        assert_eq!(last, Some(session_id.to_string()));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_no_last_session_when_empty() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_empty_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        let last = store.last_session().unwrap();
        assert_eq!(last, None);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_empty_session_has_no_messages() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_empty_msgs_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        let session_id = "empty-session";
        store.create_session(session_id).unwrap();

        let msgs = store.load_messages(session_id).unwrap();
        assert!(msgs.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_multiple_sessions() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_multi_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        store.create_session("sess-1").unwrap();
        store.create_session("sess-2").unwrap();
        store.create_session("sess-3").unwrap();

        store.save_message("sess-2", "user", "test").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 3);

        // sess-2 should be last (updated_at bumped by save_message)
        let last = store.last_session().unwrap();
        assert_eq!(last, Some("sess-2".to_string()));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_session_message_count() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_count_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        store.create_session("sess").unwrap();
        store.save_message("sess", "user", "1").unwrap();
        store.save_message("sess", "assistant", "2").unwrap();
        store.save_message("sess", "user", "3").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions[0].message_count, 3);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_update_title() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_title_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        store.create_session("sess").unwrap();
        store.update_title("sess", "My Session").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions[0].title.as_deref(), Some("My Session"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_delete_session() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_del_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        store.create_session("sess").unwrap();
        store.save_message("sess", "user", "msg").unwrap();
        store.delete_session("sess").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert!(sessions.is_empty());

        let last = store.last_session().unwrap();
        assert_eq!(last, None);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_idempotent_create() {
        let tmp = std::env::temp_dir().join(format!("aibsd_test_idem_{}.db", std::process::id()));
        let store = SessionStore::new(tmp.to_str().unwrap()).unwrap();

        store.create_session("sess").unwrap();
        store.create_session("sess").unwrap(); // should not error (INSERT OR IGNORE)

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        let _ = std::fs::remove_file(&tmp);
    }
}
