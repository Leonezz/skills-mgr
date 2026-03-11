use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    name TEXT
);

CREATE TABLE IF NOT EXISTS project_profiles (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    profile_name TEXT NOT NULL,
    activated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, profile_name)
);

CREATE TABLE IF NOT EXISTS project_agents (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    agent_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (project_id, agent_name)
);

CREATE TABLE IF NOT EXISTS placements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    skill_name TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    target_path TEXT NOT NULL,
    placed_at TEXT NOT NULL,
    UNIQUE (project_id, skill_name, agent_name)
);

CREATE TABLE IF NOT EXISTS placement_profiles (
    placement_id INTEGER NOT NULL REFERENCES placements(id) ON DELETE CASCADE,
    profile_name TEXT NOT NULL,
    PRIMARY KEY (placement_id, profile_name)
);

CREATE TABLE IF NOT EXISTS operation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    source TEXT NOT NULL,
    agent_name TEXT,
    operation TEXT NOT NULL,
    params TEXT,
    project_path TEXT,
    result TEXT NOT NULL,
    details TEXT
);
"#;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}?mode=rwc", path.display()))?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5))
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open database at {}", path.display()))?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn open_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::raw_sql(SCHEMA).execute(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // --- Projects ---

    pub async fn get_or_create_project(&self, path: &str, name: Option<&str>) -> Result<i64> {
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM projects WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id,)) = existing {
            return Ok(id);
        }

        let result = sqlx::query(
            "INSERT INTO projects (path, name) VALUES (?, ?)",
        )
        .bind(path)
        .bind(name)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    // --- Operation Log ---

    pub async fn log_operation(
        &self,
        source: &str,
        agent_name: Option<&str>,
        operation: &str,
        params: Option<&str>,
        project_path: Option<&str>,
        result: &str,
        details: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        sqlx::query(
            "INSERT INTO operation_log (timestamp, source, agent_name, operation, params, project_path, result, details)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&now)
        .bind(source)
        .bind(agent_name)
        .bind(operation)
        .bind(params)
        .bind(project_path)
        .bind(result)
        .bind(details)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_recent_logs(&self, limit: i64) -> Result<Vec<LogEntry>> {
        let rows = sqlx::query_as::<_, LogEntry>(
            "SELECT id, timestamp, source, agent_name, operation, params, project_path, result, details
             FROM operation_log ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Placements ---

    pub async fn insert_placement(
        &self,
        project_id: i64,
        skill_name: &str,
        agent_name: &str,
        target_path: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let result = sqlx::query(
            "INSERT INTO placements (project_id, skill_name, agent_name, target_path, placed_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (project_id, skill_name, agent_name) DO UPDATE SET placed_at = excluded.placed_at
             RETURNING id",
        )
        .bind(project_id)
        .bind(skill_name)
        .bind(agent_name)
        .bind(target_path)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        Ok(sqlx::Row::get::<i64, _>(&result, 0))
    }

    pub async fn link_placement_profile(&self, placement_id: i64, profile_name: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO placement_profiles (placement_id, profile_name) VALUES (?, ?)",
        )
        .bind(placement_id)
        .bind(profile_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unlink_placement_profile(&self, placement_id: i64, profile_name: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM placement_profiles WHERE placement_id = ? AND profile_name = ?",
        )
        .bind(placement_id)
        .bind(profile_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_placement_profile_count(&self, placement_id: i64) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM placement_profiles WHERE placement_id = ?",
        )
        .bind(placement_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn delete_placement(&self, placement_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM placements WHERE id = ?")
            .bind(placement_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_placements_for_project_profile(
        &self,
        project_id: i64,
        profile_name: &str,
    ) -> Result<Vec<PlacementRow>> {
        let rows = sqlx::query_as::<_, PlacementRow>(
            "SELECT p.id, p.project_id, p.skill_name, p.agent_name, p.target_path, p.placed_at
             FROM placements p
             JOIN placement_profiles pp ON p.id = pp.placement_id
             WHERE p.project_id = ? AND pp.profile_name = ?",
        )
        .bind(project_id)
        .bind(profile_name)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_all_placements_for_project(&self, project_id: i64) -> Result<Vec<PlacementRow>> {
        let rows = sqlx::query_as::<_, PlacementRow>(
            "SELECT id, project_id, skill_name, agent_name, target_path, placed_at
             FROM placements WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_placements_for_skill(&self, skill_name: &str) -> Result<Vec<PlacementRow>> {
        let rows = sqlx::query_as::<_, PlacementRow>(
            "SELECT id, project_id, skill_name, agent_name, target_path, placed_at
             FROM placements WHERE skill_name = ?",
        )
        .bind(skill_name)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn find_conflict(&self, project_id: i64, target_path: &str) -> Result<Option<PlacementRow>> {
        let row = sqlx::query_as::<_, PlacementRow>(
            "SELECT id, project_id, skill_name, agent_name, target_path, placed_at
             FROM placements WHERE project_id = ? AND target_path = ?",
        )
        .bind(project_id)
        .bind(target_path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Project Profiles ---

    pub async fn activate_project_profile(&self, project_id: i64, profile_name: &str) -> Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO project_profiles (project_id, profile_name, activated_at) VALUES (?, ?, ?)",
        )
        .bind(project_id)
        .bind(profile_name)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn deactivate_project_profile(&self, project_id: i64, profile_name: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM project_profiles WHERE project_id = ? AND profile_name = ?",
        )
        .bind(project_id)
        .bind(profile_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Project Agents ---

    pub async fn set_agent_enabled(&self, project_id: i64, agent_name: &str, enabled: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO project_agents (project_id, agent_name, enabled) VALUES (?, ?, ?)
             ON CONFLICT (project_id, agent_name) DO UPDATE SET enabled = excluded.enabled",
        )
        .bind(project_id)
        .bind(agent_name)
        .bind(enabled as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn is_agent_enabled(&self, project_id: i64, agent_name: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT enabled FROM project_agents WHERE project_id = ? AND agent_name = ?",
        )
        .bind(project_id)
        .bind(agent_name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(e,)| e != 0).unwrap_or(true))
    }

    pub async fn get_active_profiles(&self, project_id: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT profile_name FROM project_profiles WHERE project_id = ? ORDER BY activated_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct PlacementRow {
    pub id: i64,
    pub project_id: i64,
    pub skill_name: String,
    pub agent_name: String,
    pub target_path: String,
    pub placed_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub source: String,
    pub agent_name: Option<String>,
    pub operation: String,
    pub params: Option<String>,
    pub project_path: Option<String>,
    pub result: String,
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_memory_db() {
        let db = Database::open_memory().await.unwrap();
        assert!(db.pool().acquire().await.is_ok());
    }

    #[tokio::test]
    async fn test_create_project() {
        let db = Database::open_memory().await.unwrap();
        let id = db.get_or_create_project("/tmp/my-project", Some("my-project")).await.unwrap();
        assert!(id > 0);

        // Calling again returns same ID
        let id2 = db.get_or_create_project("/tmp/my-project", None).await.unwrap();
        assert_eq!(id, id2);
    }

    #[tokio::test]
    async fn test_log_operation() {
        let db = Database::open_memory().await.unwrap();
        db.log_operation("cli", None, "profile_activate", Some(r#"{"name":"rust"}"#), Some("/tmp/proj"), "success", Some("Activated")).await.unwrap();

        let logs = db.get_recent_logs(10).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].operation, "profile_activate");
        assert_eq!(logs[0].source, "cli");
    }

    #[tokio::test]
    async fn test_open_file_db() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("local").join("skills-mgr.db");
        let db = Database::open(&db_path).await.unwrap();
        let id = db.get_or_create_project("/test", None).await.unwrap();
        assert!(id > 0);
    }
}
