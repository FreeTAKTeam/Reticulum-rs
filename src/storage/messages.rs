use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub content: String,
    pub timestamp: i64,
    pub direction: String,
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

    pub fn insert_message(&self, record: &MessageRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO messages (id, source, destination, content, timestamp, direction) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.id,
                record.source,
                record.destination,
                record.content,
                record.timestamp,
                record.direction,
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
                "SELECT id, source, destination, content, timestamp, direction FROM messages WHERE timestamp < ?1 ORDER BY timestamp DESC LIMIT ?2",
            )?;
            let mut rows = stmt.query(params![ts, limit as i64])?;
            while let Some(row) = rows.next()? {
                records.push(MessageRecord {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    destination: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    direction: row.get(5)?,
                });
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, source, destination, content, timestamp, direction FROM messages ORDER BY timestamp DESC LIMIT ?1",
            )?;
            let mut rows = stmt.query(params![limit as i64])?;
            while let Some(row) = rows.next()? {
                records.push(MessageRecord {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    destination: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    direction: row.get(5)?,
                });
            }
        }
        Ok(records)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                destination TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                direction TEXT NOT NULL
            );",
        )?;
        Ok(())
    }
}
