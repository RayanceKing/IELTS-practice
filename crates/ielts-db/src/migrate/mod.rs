use std::sync::OnceLock;

use rusqlite::Connection;

use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

fn migrations() -> &'static [Migration] {
    static MIGRATIONS: OnceLock<Vec<Migration>> = OnceLock::new();
    MIGRATIONS.get_or_init(|| {
        vec![Migration {
            version: 1,
            name: "v2_core",
            sql: include_str!("../../migrations/0001_v2_core.sql"),
        }]
    })
}

pub fn current_version(conn: &Connection) -> DbResult<i64> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

/// Apply all pending migrations inside individual transactions.
pub fn migrate(conn: &mut Connection) -> DbResult<Vec<i64>> {
    let mut applied = Vec::new();
    let mut version = current_version(conn)?;
    for migration in migrations() {
        if migration.version <= version {
            continue;
        }
        if migration.version != version + 1 {
            return Err(DbError::Migration(format!(
                "migration gap: have {version}, next {}",
                migration.version
            )));
        }
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                migration.version,
                migration.name,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        tx.commit()?;
        applied.push(migration.version);
        version = migration.version;
    }
    Ok(applied)
}

/// Re-run migrate on a fresh connection to assert idempotency.
pub fn verify_idempotent(conn: &mut Connection) -> DbResult<()> {
    let before = current_version(conn)?;
    let applied = migrate(conn)?;
    if !applied.is_empty() {
        return Err(DbError::Migration(format!(
            "expected no migrations, applied {applied:?}"
        )));
    }
    let after = current_version(conn)?;
    if before != after {
        return Err(DbError::Migration(format!(
            "version changed on idempotent verify: {before} -> {after}"
        )));
    }
    Ok(())
}

pub fn open_and_migrate(path: impl AsRef<std::path::Path>) -> DbResult<Connection> {
    let mut conn = crate::sqlite::open_connection(&crate::sqlite::DbOpenOptions::create(
        path.as_ref().to_path_buf(),
    ))?;
    migrate(&mut conn)?;
    Ok(conn)
}
