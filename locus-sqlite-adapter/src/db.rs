use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result};
use rusqlite::Connection;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS temporal_node (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL DEFAULT '',
    session_id TEXT NOT NULL,
    raw TEXT NOT NULL,
    tier TEXT NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    compression_depth INTEGER NOT NULL,
    parent_node_id TEXT,
    sync_key TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    source_metadata TEXT,
    context_summary TEXT,
    semantic_tags TEXT,
    semantic_links TEXT,
    embedding TEXT,
    embedding_model TEXT,
    embedding_dimensions INTEGER,
    embedded_at_ms INTEGER,
    psi REAL NOT NULL,
    rho REAL NOT NULL,
    kappa REAL NOT NULL,
    user_stability REAL NOT NULL,
    user_friction REAL NOT NULL,
    user_logic REAL NOT NULL,
    user_autonomy REAL NOT NULL,
    user_psi REAL NOT NULL,
    model_stability REAL NOT NULL,
    model_friction REAL NOT NULL,
    model_logic REAL NOT NULL,
    model_autonomy REAL NOT NULL,
    model_psi REAL NOT NULL,
    comp_stability REAL NOT NULL,
    comp_friction REAL NOT NULL,
    comp_logic REAL NOT NULL,
    comp_autonomy REAL NOT NULL,
    comp_psi REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS calibration (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL DEFAULT '',
    session_id TEXT NOT NULL,
    stability REAL NOT NULL,
    friction REAL NOT NULL,
    logic REAL NOT NULL,
    autonomy REAL NOT NULL,
    psi REAL NOT NULL,
    trigger TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_checkpoint (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL DEFAULT '',
    session_id TEXT NOT NULL,
    connector_id TEXT NOT NULL,
    cursor_updated_at_ms INTEGER,
    cursor_sync_key TEXT,
    metadata TEXT,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS semantic_tag_index (
    tenant_id TEXT NOT NULL DEFAULT '',
    session_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    sync_key TEXT NOT NULL,
    tag TEXT NOT NULL,
    embedding TEXT,
    embedding_model TEXT,
    embedding_dimensions INTEGER,
    embedded_at_ms INTEGER,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, sync_key, tag)
);

CREATE INDEX IF NOT EXISTS idx_node_session ON temporal_node (session_id);
CREATE INDEX IF NOT EXISTS idx_node_tenant_session ON temporal_node (tenant_id, session_id);
CREATE INDEX IF NOT EXISTS idx_node_tier ON temporal_node (tier);
CREATE INDEX IF NOT EXISTS idx_node_timestamp ON temporal_node (timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_node_change_cursor ON temporal_node (tenant_id, session_id, updated_at_ms, sync_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_node_sync_identity ON temporal_node (tenant_id, session_id, sync_key);
CREATE INDEX IF NOT EXISTS idx_tag_lookup ON semantic_tag_index (tenant_id, tag);
CREATE INDEX IF NOT EXISTS idx_tag_node ON semantic_tag_index (tenant_id, node_id);
CREATE INDEX IF NOT EXISTS idx_cal_session ON calibration (session_id);
CREATE INDEX IF NOT EXISTS idx_cal_tenant_session ON calibration (tenant_id, session_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_checkpoint_scope ON sync_checkpoint (tenant_id, session_id, connector_id);
"#;

/// Shared SQLite connection for the node store and semantic index.
#[derive(Debug)]
pub struct SqliteDatabase {
    conn: Mutex<Connection>,
}

impl SqliteDatabase {
    /// Open or create a database file. Parent directories are created when missing.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite database {}", path.display()))?;
        configure(&conn, true)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open a private in-memory database. Used by tests and ephemeral hosts.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory sqlite")?;
        configure(&conn, false)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create tables and indexes. Safe to call more than once.
    pub fn initialize(&self) -> Result<()> {
        self.with_connection(|conn| {
            conn.execute_batch(SCHEMA)
                .context("failed to initialize sqlite schema")?;
            Ok(())
        })
    }

    pub(crate) fn with_connection<T>(
        &self,
        body: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<T> {
        let guard = self.lock()?;
        body(&guard)
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow::anyhow!("sqlite connection lock poisoned"))
    }
}

fn configure(conn: &Connection, wal: bool) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .context("failed to set sqlite busy timeout")?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("failed to enable sqlite foreign keys")?;
    if wal {
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable sqlite WAL")?;
    }
    Ok(())
}
