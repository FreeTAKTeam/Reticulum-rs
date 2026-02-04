use rusqlite::{params, Connection};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub title: String,
    pub content: String,
    pub timestamp: i64,
    pub direction: String,
    pub fields: Option<JsonValue>,
    pub receipt_status: Option<String>,
}

pub struct MessagesStore {
    conn: Connection,
}

impl MessagesStore {
    pub fn in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open(path: &std::path::Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn insert_message(&self, record: &MessageRecord) -> rusqlite::Result<()> {
        let fields_json = record
            .fields
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_default());
        self.conn.execute(
            "INSERT OR REPLACE INTO messages (id, source, destination, title, content, timestamp, direction, fields, receipt_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.source,
                record.destination,
                record.title,
                record.content,
                record.timestamp,
                record.direction,
                fields_json,
                record.receipt_status,
            ],
        )?;
        Ok(())
    }

    pub fn list_messages(
        &self,
        limit: usize,
        before_ts: Option<i64>,
    ) -> rusqlite::Result<Vec<MessageRecord>> {
        let mut records = Vec::new();
        if let Some(ts) = before_ts {
            let mut stmt = self.conn.prepare(
                "SELECT id, source, destination, title, content, timestamp, direction, fields, receipt_status FROM messages WHERE timestamp < ?1 ORDER BY timestamp DESC LIMIT ?2",
            )?;
            let mut rows = stmt.query(params![ts, limit as i64])?;
            while let Some(row) = rows.next()? {
                let fields_json: Option<String> = row.get(7)?;
                let fields = fields_json
                    .as_ref()
                    .and_then(|value| serde_json::from_str(value).ok());
                let receipt_status: Option<String> = row.get(8)?;
                records.push(MessageRecord {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    destination: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    timestamp: row.get(5)?,
                    direction: row.get(6)?,
                    fields,
                    receipt_status,
                });
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, source, destination, title, content, timestamp, direction, fields, receipt_status FROM messages ORDER BY timestamp DESC LIMIT ?1",
            )?;
            let mut rows = stmt.query(params![limit as i64])?;
            while let Some(row) = rows.next()? {
                let fields_json: Option<String> = row.get(7)?;
                let fields = fields_json
                    .as_ref()
                    .and_then(|value| serde_json::from_str(value).ok());
                let receipt_status: Option<String> = row.get(8)?;
                records.push(MessageRecord {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    destination: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    timestamp: row.get(5)?,
                    direction: row.get(6)?,
                    fields,
                    receipt_status,
                });
            }
        }
        Ok(records)
    }

    pub fn update_receipt_status(
        &self,
        message_id: &str,
        status: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE messages SET receipt_status = ?1 WHERE id = ?2",
            params![status, message_id],
        )?;
        Ok(())
    }

    pub fn clear_messages(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM messages", [])?;
        Ok(())
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                destination TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                direction TEXT NOT NULL,
                fields TEXT,
                receipt_status TEXT
            );",
        )?;
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN title TEXT", []);
        let _ = self
            .conn
            .execute("UPDATE messages SET title = '' WHERE title IS NULL", []);
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN fields TEXT", []);
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN receipt_status TEXT", []);
        Ok(())
    }
}
