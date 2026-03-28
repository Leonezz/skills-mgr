use anyhow::{Context, Result};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, Statement,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::entity::{
    operation_log, placement_profiles, placements, project_agents, project_linked_profiles,
    project_profiles, projects,
};

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

CREATE TABLE IF NOT EXISTS project_linked_profiles (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    profile_name TEXT NOT NULL,
    linked_at TEXT NOT NULL,
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
    conn: DatabaseConnection,
}

impl Database {
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options =
            SqliteConnectOptions::from_str(&format!("sqlite:{}?mode=rwc", path.display()))?
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .busy_timeout(std::time::Duration::from_secs(5))
                .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open database at {}", path.display()))?;

        let conn = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
        let db = Self { conn };
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

        let conn = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
        let db = Self { conn };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        let backend = self.conn.get_database_backend();
        for stmt in SCHEMA.split(';') {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() {
                self.conn
                    .execute(Statement::from_string(backend, trimmed.to_string()))
                    .await?;
            }
        }
        Ok(())
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    // --- Projects ---

    pub async fn get_or_create_project(&self, path: &str, name: Option<&str>) -> Result<i64> {
        let existing = projects::Entity::find()
            .filter(projects::Column::Path.eq(path))
            .one(&self.conn)
            .await?;

        if let Some(project) = existing {
            return Ok(project.id);
        }

        let model = projects::ActiveModel {
            path: Set(path.to_string()),
            name: Set(name.map(|n| n.to_string())),
            ..Default::default()
        };
        let result = projects::Entity::insert(model).exec(&self.conn).await?;
        Ok(result.last_insert_id)
    }

    pub async fn list_all_projects(&self) -> Result<Vec<ProjectRow>> {
        let rows = projects::Entity::find()
            .order_by_asc(projects::Column::Id)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(ProjectRow::from).collect())
    }

    pub async fn delete_project(&self, project_id: i64) -> Result<()> {
        placements::Entity::delete_many()
            .filter(placements::Column::ProjectId.eq(project_id))
            .exec(&self.conn)
            .await?;
        project_profiles::Entity::delete_many()
            .filter(project_profiles::Column::ProjectId.eq(project_id))
            .exec(&self.conn)
            .await?;
        project_linked_profiles::Entity::delete_many()
            .filter(project_linked_profiles::Column::ProjectId.eq(project_id))
            .exec(&self.conn)
            .await?;
        project_agents::Entity::delete_many()
            .filter(project_agents::Column::ProjectId.eq(project_id))
            .exec(&self.conn)
            .await?;
        projects::Entity::delete_by_id(project_id)
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    // --- Operation Log ---

    #[allow(clippy::too_many_arguments)]
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
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let model = operation_log::ActiveModel {
            timestamp: Set(now),
            source: Set(source.to_string()),
            agent_name: Set(agent_name.map(|s| s.to_string())),
            operation: Set(operation.to_string()),
            params: Set(params.map(|s| s.to_string())),
            project_path: Set(project_path.map(|s| s.to_string())),
            result: Set(result.to_string()),
            details: Set(details.map(|s| s.to_string())),
            ..Default::default()
        };
        operation_log::Entity::insert(model)
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_recent_logs(&self, limit: i64) -> Result<Vec<LogEntry>> {
        let rows = operation_log::Entity::find()
            .order_by_desc(operation_log::Column::Id)
            .limit(limit as u64)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(LogEntry::from).collect())
    }

    // --- Placements ---

    pub async fn insert_placement(
        &self,
        project_id: i64,
        skill_name: &str,
        agent_name: &str,
        target_path: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let existing = placements::Entity::find()
            .filter(placements::Column::ProjectId.eq(project_id))
            .filter(placements::Column::SkillName.eq(skill_name))
            .filter(placements::Column::AgentName.eq(agent_name))
            .one(&self.conn)
            .await?;

        if let Some(row) = existing {
            let id = row.id;
            let mut active: placements::ActiveModel = row.into();
            active.placed_at = Set(now);
            active.update(&self.conn).await?;
            Ok(id)
        } else {
            let model = placements::ActiveModel {
                project_id: Set(project_id),
                skill_name: Set(skill_name.to_string()),
                agent_name: Set(agent_name.to_string()),
                target_path: Set(target_path.to_string()),
                placed_at: Set(now),
                ..Default::default()
            };
            let result = placements::Entity::insert(model).exec(&self.conn).await?;
            Ok(result.last_insert_id)
        }
    }

    pub async fn link_placement_profile(
        &self,
        placement_id: i64,
        profile_name: &str,
    ) -> Result<()> {
        let existing = placement_profiles::Entity::find()
            .filter(placement_profiles::Column::PlacementId.eq(placement_id))
            .filter(placement_profiles::Column::ProfileName.eq(profile_name))
            .one(&self.conn)
            .await?;

        if existing.is_none() {
            let model = placement_profiles::ActiveModel {
                placement_id: Set(placement_id),
                profile_name: Set(profile_name.to_string()),
            };
            placement_profiles::Entity::insert(model)
                .exec(&self.conn)
                .await?;
        }
        Ok(())
    }

    pub async fn unlink_placement_profile(
        &self,
        placement_id: i64,
        profile_name: &str,
    ) -> Result<()> {
        placement_profiles::Entity::delete_many()
            .filter(placement_profiles::Column::PlacementId.eq(placement_id))
            .filter(placement_profiles::Column::ProfileName.eq(profile_name))
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_placement_profile_count(&self, placement_id: i64) -> Result<i64> {
        let count = placement_profiles::Entity::find()
            .filter(placement_profiles::Column::PlacementId.eq(placement_id))
            .count(&self.conn)
            .await?;
        Ok(count as i64)
    }

    pub async fn delete_placement(&self, placement_id: i64) -> Result<()> {
        placements::Entity::delete_by_id(placement_id)
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_placements_for_project_profile(
        &self,
        project_id: i64,
        profile_name: &str,
    ) -> Result<Vec<PlacementRow>> {
        let rows = placements::Entity::find()
            .join(
                JoinType::InnerJoin,
                placements::Relation::PlacementProfiles.def(),
            )
            .filter(placements::Column::ProjectId.eq(project_id))
            .filter(placement_profiles::Column::ProfileName.eq(profile_name))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(PlacementRow::from).collect())
    }

    pub async fn get_all_placements_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<PlacementRow>> {
        let rows = placements::Entity::find()
            .filter(placements::Column::ProjectId.eq(project_id))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(PlacementRow::from).collect())
    }

    pub async fn get_placements_for_skill(&self, skill_name: &str) -> Result<Vec<PlacementRow>> {
        let rows = placements::Entity::find()
            .filter(placements::Column::SkillName.eq(skill_name))
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(PlacementRow::from).collect())
    }

    pub async fn find_conflict(
        &self,
        project_id: i64,
        target_path: &str,
    ) -> Result<Option<PlacementRow>> {
        let row = placements::Entity::find()
            .filter(placements::Column::ProjectId.eq(project_id))
            .filter(placements::Column::TargetPath.eq(target_path))
            .one(&self.conn)
            .await?;
        Ok(row.map(PlacementRow::from))
    }

    // --- Project Profiles ---

    pub async fn activate_project_profile(
        &self,
        project_id: i64,
        profile_name: &str,
    ) -> Result<()> {
        let existing = project_profiles::Entity::find()
            .filter(project_profiles::Column::ProjectId.eq(project_id))
            .filter(project_profiles::Column::ProfileName.eq(profile_name))
            .one(&self.conn)
            .await?;

        if existing.is_none() {
            let now = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            let model = project_profiles::ActiveModel {
                project_id: Set(project_id),
                profile_name: Set(profile_name.to_string()),
                activated_at: Set(now),
            };
            project_profiles::Entity::insert(model)
                .exec(&self.conn)
                .await?;
        }
        Ok(())
    }

    pub async fn deactivate_project_profile(
        &self,
        project_id: i64,
        profile_name: &str,
    ) -> Result<()> {
        project_profiles::Entity::delete_many()
            .filter(project_profiles::Column::ProjectId.eq(project_id))
            .filter(project_profiles::Column::ProfileName.eq(profile_name))
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    // --- Project Agents ---

    pub async fn set_agent_enabled(
        &self,
        project_id: i64,
        agent_name: &str,
        enabled: bool,
    ) -> Result<()> {
        let existing = project_agents::Entity::find()
            .filter(project_agents::Column::ProjectId.eq(project_id))
            .filter(project_agents::Column::AgentName.eq(agent_name))
            .one(&self.conn)
            .await?;

        if let Some(row) = existing {
            let mut active: project_agents::ActiveModel = row.into();
            active.enabled = Set(enabled as i32);
            active.update(&self.conn).await?;
        } else {
            let model = project_agents::ActiveModel {
                project_id: Set(project_id),
                agent_name: Set(agent_name.to_string()),
                enabled: Set(enabled as i32),
            };
            project_agents::Entity::insert(model)
                .exec(&self.conn)
                .await?;
        }
        Ok(())
    }

    pub async fn is_agent_enabled(&self, project_id: i64, agent_name: &str) -> Result<bool> {
        let row = project_agents::Entity::find()
            .filter(project_agents::Column::ProjectId.eq(project_id))
            .filter(project_agents::Column::AgentName.eq(agent_name))
            .one(&self.conn)
            .await?;
        Ok(row.map(|r| r.enabled != 0).unwrap_or(true))
    }

    pub async fn get_active_profiles(&self, project_id: i64) -> Result<Vec<String>> {
        let rows = project_profiles::Entity::find()
            .filter(project_profiles::Column::ProjectId.eq(project_id))
            .order_by_asc(project_profiles::Column::ActivatedAt)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| r.profile_name).collect())
    }

    pub async fn get_projects_for_profile(
        &self,
        profile_name: &str,
    ) -> Result<Vec<(String, Option<String>)>> {
        let rows = projects::Entity::find()
            .join(
                JoinType::InnerJoin,
                projects::Relation::ProjectProfiles.def(),
            )
            .filter(project_profiles::Column::ProfileName.eq(profile_name))
            .order_by_asc(project_profiles::Column::ActivatedAt)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| (r.path, r.name)).collect())
    }

    // --- Project Linked Profiles ---

    pub async fn link_profile_to_project(&self, project_id: i64, profile_name: &str) -> Result<()> {
        let existing = project_linked_profiles::Entity::find()
            .filter(project_linked_profiles::Column::ProjectId.eq(project_id))
            .filter(project_linked_profiles::Column::ProfileName.eq(profile_name))
            .one(&self.conn)
            .await?;

        if existing.is_none() {
            let now = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            let model = project_linked_profiles::ActiveModel {
                project_id: Set(project_id),
                profile_name: Set(profile_name.to_string()),
                linked_at: Set(now),
            };
            project_linked_profiles::Entity::insert(model)
                .exec(&self.conn)
                .await?;
        }
        Ok(())
    }

    pub async fn unlink_profile_from_project(
        &self,
        project_id: i64,
        profile_name: &str,
    ) -> Result<()> {
        project_linked_profiles::Entity::delete_many()
            .filter(project_linked_profiles::Column::ProjectId.eq(project_id))
            .filter(project_linked_profiles::Column::ProfileName.eq(profile_name))
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_linked_profiles(&self, project_id: i64) -> Result<Vec<String>> {
        let rows = project_linked_profiles::Entity::find()
            .filter(project_linked_profiles::Column::ProjectId.eq(project_id))
            .order_by_asc(project_linked_profiles::Column::LinkedAt)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(|r| r.profile_name).collect())
    }

    /// Collect all placed skill target paths across all projects.
    /// Returns a HashSet of canonicalized absolute paths for use in discovery filtering.
    pub async fn collect_placed_paths(&self) -> Result<std::collections::HashSet<String>> {
        let projects = self.list_all_projects().await?;
        let mut paths = std::collections::HashSet::new();
        for project in &projects {
            let placements = self.get_all_placements_for_project(project.id).await?;
            for p in placements {
                // Canonicalize for reliable comparison (e.g. /var → /private/var on macOS)
                let canonical = std::fs::canonicalize(&p.target_path)
                    .map(|c| c.to_string_lossy().to_string())
                    .unwrap_or(p.target_path);
                paths.insert(canonical);
            }
        }
        Ok(paths)
    }
}

#[derive(Debug)]
pub struct ProjectRow {
    pub id: i64,
    pub path: String,
    pub name: Option<String>,
}

impl From<projects::Model> for ProjectRow {
    fn from(m: projects::Model) -> Self {
        Self {
            id: m.id,
            path: m.path,
            name: m.name,
        }
    }
}

#[derive(Debug)]
pub struct PlacementRow {
    pub id: i64,
    pub project_id: i64,
    pub skill_name: String,
    pub agent_name: String,
    pub target_path: String,
    pub placed_at: String,
}

impl From<placements::Model> for PlacementRow {
    fn from(m: placements::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            skill_name: m.skill_name,
            agent_name: m.agent_name,
            target_path: m.target_path,
            placed_at: m.placed_at,
        }
    }
}

#[derive(Debug)]
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

impl From<operation_log::Model> for LogEntry {
    fn from(m: operation_log::Model) -> Self {
        Self {
            id: m.id,
            timestamp: m.timestamp,
            source: m.source,
            agent_name: m.agent_name,
            operation: m.operation,
            params: m.params,
            project_path: m.project_path,
            result: m.result,
            details: m.details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_memory_db() {
        let db = Database::open_memory().await.unwrap();
        let projects = db.list_all_projects().await.unwrap();
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_create_project() {
        let db = Database::open_memory().await.unwrap();
        let id = db
            .get_or_create_project("/tmp/my-project", Some("my-project"))
            .await
            .unwrap();
        assert!(id > 0);

        // Calling again returns same ID
        let id2 = db
            .get_or_create_project("/tmp/my-project", None)
            .await
            .unwrap();
        assert_eq!(id, id2);
    }

    #[tokio::test]
    async fn test_log_operation() {
        let db = Database::open_memory().await.unwrap();
        db.log_operation(
            "cli",
            None,
            "profile_activate",
            Some(r#"{"name":"rust"}"#),
            Some("/tmp/proj"),
            "success",
            Some("Activated"),
        )
        .await
        .unwrap();

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
