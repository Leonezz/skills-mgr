# Skills-Mgr Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-agent skill management tool with Tauri 2 GUI, CLI, and MCP server that manages composable skill profiles using the Agent Skills open standard.

**Architecture:** Rust workspace with 4 crates — `skills-core` (shared library), `skills-cli` (clap binary), `skills-mcp` (rmcp module), `skills-gui` (Tauri 2 + React). All business logic lives in `skills-core`; the other crates are thin wrappers. SQLite (WAL mode) for project-level state. TOML config files for git-trackable definitions.

**Tech Stack:** Rust 1.92+, Tauri 2, React 19, TypeScript, Tailwind CSS, shadcn/ui, sqlx (SQLite), clap, rmcp, serde, toml, sha2, git2

**Spec:** `docs/superpowers/specs/2026-03-11-skills-mgr-design.md`

---

## Chunk 1: Workspace Foundation + Config Layer

### Task 1: Initialize Rust Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/skills-core/Cargo.toml`
- Create: `crates/skills-core/src/lib.rs`
- Create: `crates/skills-cli/Cargo.toml`
- Create: `crates/skills-cli/src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/skills-core",
    "crates/skills-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Create skills-core crate**

`crates/skills-core/Cargo.toml`:
```toml
[package]
name = "skills-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
sha2.workspace = true
chrono.workspace = true
thiserror.workspace = true
anyhow.workspace = true
```

`crates/skills-core/src/lib.rs`:
```rust
pub mod config;

pub use config::AppDirs;
```

- [ ] **Step 3: Create skills-cli crate**

`crates/skills-cli/Cargo.toml`:
```toml
[package]
name = "skills-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "skills-mgr"
path = "src/main.rs"

[dependencies]
skills-core = { path = "../skills-core" }
clap = { version = "4", features = ["derive"] }
anyhow.workspace = true
```

`crates/skills-cli/src/main.rs`:
```rust
fn main() {
    println!("skills-mgr v0.1.0");
}
```

- [ ] **Step 4: Create .gitignore**

```
/target
.DS_Store
```

- [ ] **Step 5: Verify workspace builds**

Run: `cargo build`
Expected: Compiles successfully, produces `target/debug/skills-mgr` binary

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/ .gitignore
git commit -m "feat: initialize Rust workspace with skills-core and skills-cli"
```

---

### Task 2: App Directory Structure + Config Types

**Files:**
- Create: `crates/skills-core/src/config.rs`
- Modify: `crates/skills-core/src/lib.rs`

The config module handles locating `~/.skills-mgr/`, parsing TOML config files, and providing typed access to all configuration. This is the foundation everything else builds on.

- [ ] **Step 1: Write tests for AppDirs**

Create `crates/skills-core/src/config.rs`:
```rust
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Root directory structure for skills-mgr.
/// All paths derived from a single base directory (default: ~/.skills-mgr/).
#[derive(Debug, Clone)]
pub struct AppDirs {
    base: PathBuf,
}

impl AppDirs {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    pub fn default_base() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".skills-mgr"))
    }

    pub fn base(&self) -> &Path { &self.base }
    pub fn registry(&self) -> PathBuf { self.base.join("registry") }
    pub fn sources_toml(&self) -> PathBuf { self.base.join("sources.toml") }
    pub fn profiles_toml(&self) -> PathBuf { self.base.join("profiles.toml") }
    pub fn agents_toml(&self) -> PathBuf { self.base.join("agents.toml") }
    pub fn local(&self) -> PathBuf { self.base.join("local") }
    pub fn database(&self) -> PathBuf { self.base.join("local").join("skills-mgr.db") }
    pub fn cache(&self) -> PathBuf { self.base.join("local").join("cache") }

    /// Ensure all required directories exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.registry())?;
        std::fs::create_dir_all(self.local())?;
        std::fs::create_dir_all(self.cache())?;
        Ok(())
    }
}

// --- TOML Config Types ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesConfig {
    #[serde(default)]
    pub skills: std::collections::BTreeMap<String, SkillSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSource {
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub url: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub hash: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Git,
    Registry,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesConfig {
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub profiles: std::collections::BTreeMap<String, ProfileDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseConfig {
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDef {
    pub description: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub agents: std::collections::BTreeMap<String, AgentDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub project_path: String,
    pub global_path: String,
}

impl SourcesConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl ProfilesConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl AgentsConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_app_dirs_paths() {
        let base = PathBuf::from("/tmp/test-skills-mgr");
        let dirs = AppDirs::new(base.clone());
        assert_eq!(dirs.registry(), base.join("registry"));
        assert_eq!(dirs.database(), base.join("local").join("skills-mgr.db"));
        assert_eq!(dirs.profiles_toml(), base.join("profiles.toml"));
    }

    #[test]
    fn test_ensure_dirs_creates_structure() {
        let tmp = TempDir::new().unwrap();
        let dirs = AppDirs::new(tmp.path().to_path_buf());
        dirs.ensure_dirs().unwrap();
        assert!(dirs.registry().exists());
        assert!(dirs.local().exists());
        assert!(dirs.cache().exists());
    }

    #[test]
    fn test_profiles_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("profiles.toml");

        let config = ProfilesConfig {
            base: BaseConfig { skills: vec!["code-review".into(), "obsidian".into()] },
            profiles: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("rust".into(), ProfileDef {
                    description: Some("Rust development".into()),
                    skills: vec!["rust-engineer".into()],
                    includes: vec![],
                });
                m.insert("rust-react".into(), ProfileDef {
                    description: Some("Full-stack".into()),
                    skills: vec!["api-design".into()],
                    includes: vec!["rust".into()],
                });
                m
            },
        };
        config.save(&path).unwrap();
        let loaded = ProfilesConfig::load(&path).unwrap();
        assert_eq!(loaded.base.skills, vec!["code-review", "obsidian"]);
        assert_eq!(loaded.profiles["rust"].skills, vec!["rust-engineer"]);
        assert_eq!(loaded.profiles["rust-react"].includes, vec!["rust"]);
    }

    #[test]
    fn test_sources_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sources.toml");

        let config = SourcesConfig {
            skills: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("rust-engineer".into(), SkillSource {
                    source_type: SourceType::Git,
                    url: Some("https://github.com/anthropics/skills".into()),
                    path: Some("rust-engineer".into()),
                    git_ref: Some("main".into()),
                    hash: Some("sha256:abc123".into()),
                    updated_at: Some("2026-03-10T12:00:00Z".into()),
                });
                m
            },
        };
        config.save(&path).unwrap();
        let loaded = SourcesConfig::load(&path).unwrap();
        assert_eq!(loaded.skills["rust-engineer"].source_type, SourceType::Git);
    }

    #[test]
    fn test_agents_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("agents.toml");

        let config = AgentsConfig {
            agents: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("claude-code".into(), AgentDef {
                    project_path: ".claude/skills".into(),
                    global_path: "~/.claude/skills".into(),
                });
                m
            },
        };
        config.save(&path).unwrap();
        let loaded = AgentsConfig::load(&path).unwrap();
        assert_eq!(loaded.agents["claude-code"].project_path, ".claude/skills");
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let path = PathBuf::from("/nonexistent/profiles.toml");
        let config = ProfilesConfig::load(&path).unwrap();
        assert!(config.base.skills.is_empty());
        assert!(config.profiles.is_empty());
    }
}
```

- [ ] **Step 2: Add dependencies and update lib.rs**

Add `dirs = "6"` and `tempfile = "3"` (dev) to `crates/skills-core/Cargo.toml`:
```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
sha2.workspace = true
chrono.workspace = true
thiserror.workspace = true
anyhow.workspace = true
dirs = "6"

[dev-dependencies]
tempfile = "3"
```

Update `crates/skills-core/src/lib.rs`:
```rust
pub mod config;

pub use config::{
    AgentsConfig, AppDirs, ProfilesConfig, SourcesConfig,
};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p skills-core`
Expected: All 5 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/skills-core/
git commit -m "feat(core): add config types and AppDirs for TOML parsing"
```

---

### Task 3: SQLite Database Layer

**Files:**
- Create: `crates/skills-core/src/db.rs`
- Modify: `crates/skills-core/Cargo.toml`
- Modify: `crates/skills-core/src/lib.rs`

- [ ] **Step 1: Add sqlx dependency**

Add to `crates/skills-core/Cargo.toml` dependencies:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio.workspace = true
```

- [ ] **Step 2: Write database module with migrations and tests**

Create `crates/skills-core/src/db.rs`:
```rust
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
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod config;
pub mod db;

pub use config::{
    AgentsConfig, AppDirs, ProfilesConfig, SourcesConfig,
};
pub use db::Database;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p skills-core`
Expected: All tests pass (config tests + db tests)

- [ ] **Step 5: Commit**

```bash
git add crates/skills-core/
git commit -m "feat(core): add SQLite database layer with migrations and operation log"
```

---

### Task 4: Registry Module (Skill CRUD + Hashing)

**Files:**
- Create: `crates/skills-core/src/registry.rs`
- Modify: `crates/skills-core/src/lib.rs`

The registry module manages the central `~/.skills-mgr/registry/` directory — listing, adding, removing skills, computing content hashes.

- [ ] **Step 1: Write registry module with tests**

Create `crates/skills-core/src/registry.rs`:
```rust
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{AppDirs, SkillSource, SourceType, SourcesConfig};

/// Metadata parsed from a SKILL.md frontmatter.
#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: Option<String>,
    pub dir_path: PathBuf,
    pub files: Vec<String>,
    pub source: Option<SkillSource>,
}

/// Manages the skill registry directory.
pub struct Registry {
    dirs: AppDirs,
}

impl Registry {
    pub fn new(dirs: AppDirs) -> Self {
        Self { dirs }
    }

    /// List all skills in the registry.
    pub fn list(&self) -> Result<Vec<SkillMeta>> {
        let registry = self.dirs.registry();
        if !registry.exists() {
            return Ok(vec![]);
        }

        let sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        let mut skills = Vec::new();

        for entry in std::fs::read_dir(&registry)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let skill_dir = entry.path();
            let skill_md = skill_dir.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let description = parse_description(&skill_md).ok();
            let files = list_files_recursive(&skill_dir)?;
            let source = sources.skills.get(&name).cloned();

            skills.push(SkillMeta {
                name,
                description,
                dir_path: skill_dir,
                files,
                source,
            });
        }

        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    /// Get a single skill by name.
    pub fn get(&self, name: &str) -> Result<Option<SkillMeta>> {
        let skill_dir = self.dirs.registry().join(name);
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Ok(None);
        }

        let sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        let description = parse_description(&skill_md).ok();
        let files = list_files_recursive(&skill_dir)?;
        let source = sources.skills.get(name).cloned();

        Ok(Some(SkillMeta {
            name: name.to_string(),
            description,
            dir_path: skill_dir,
            files,
            source,
        }))
    }

    /// Check if a skill exists in the registry.
    pub fn exists(&self, name: &str) -> bool {
        self.dirs.registry().join(name).join("SKILL.md").exists()
    }

    /// Create a new skill with a scaffold SKILL.md.
    pub fn create(&self, name: &str, description: &str) -> Result<PathBuf> {
        let skill_dir = self.dirs.registry().join(name);
        if skill_dir.exists() {
            bail!("Skill '{}' already exists in registry", name);
        }
        std::fs::create_dir_all(&skill_dir)?;

        let content = format!(
            "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nTODO: Add skill instructions here.\n",
            name, description, name
        );
        std::fs::write(skill_dir.join("SKILL.md"), content)?;

        // Record in sources.toml as local
        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        let hash = compute_tree_hash(&skill_dir)?;
        sources.skills.insert(name.to_string(), SkillSource {
            source_type: SourceType::Local,
            url: None,
            path: None,
            git_ref: None,
            hash: Some(hash),
            updated_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
        });
        sources.save(&self.dirs.sources_toml())?;

        Ok(skill_dir)
    }

    /// Remove a skill from the registry.
    pub fn remove(&self, name: &str) -> Result<()> {
        let skill_dir = self.dirs.registry().join(name);
        if !skill_dir.exists() {
            bail!("Skill '{}' not found in registry", name);
        }
        std::fs::remove_dir_all(&skill_dir)?;

        // Remove from sources.toml
        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.remove(name);
        sources.save(&self.dirs.sources_toml())?;

        Ok(())
    }

    /// Add a skill from a local directory (copy into registry).
    pub fn add_from_local(&self, source_path: &Path) -> Result<String> {
        let skill_md = source_path.join("SKILL.md");
        if !skill_md.exists() {
            bail!("No SKILL.md found at {}", source_path.display());
        }

        let name = source_path
            .file_name()
            .context("Invalid source path")?
            .to_string_lossy()
            .to_string();

        let dest = self.dirs.registry().join(&name);
        if dest.exists() {
            bail!("Skill '{}' already exists in registry", name);
        }

        copy_dir_recursive(source_path, &dest)?;

        let hash = compute_tree_hash(&dest)?;
        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.insert(name.clone(), SkillSource {
            source_type: SourceType::Local,
            url: None,
            path: None,
            git_ref: None,
            hash: Some(hash),
            updated_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
        });
        sources.save(&self.dirs.sources_toml())?;

        Ok(name)
    }
}

/// Compute a tree hash for a skill directory.
/// SHA-256 of sorted concatenation of "relative_path:file_sha256" for all files.
pub fn compute_tree_hash(dir: &Path) -> Result<String> {
    let files = list_files_recursive(dir)?;
    let mut entries: Vec<String> = Vec::new();

    for rel_path in &files {
        let full_path = dir.join(rel_path);
        let content = std::fs::read(&full_path)
            .with_context(|| format!("Failed to read {}", full_path.display()))?;
        let file_hash = format!("{:x}", Sha256::digest(&content));
        entries.push(format!("{}:{}", rel_path, file_hash));
    }

    entries.sort();
    let combined = entries.join("\n");
    let tree_hash = format!("sha256:{:x}", Sha256::digest(combined.as_bytes()));
    Ok(tree_hash)
}

/// List all files in a directory recursively, returning relative paths sorted.
fn list_files_recursive(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    list_files_inner(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn list_files_inner(base: &Path, current: &Path, files: &mut Vec<String>) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            list_files_inner(base, &path, files)?;
        } else {
            let rel = path.strip_prefix(base)?.to_string_lossy().to_string();
            files.push(rel);
        }
    }
    Ok(())
}

/// Parse the description from SKILL.md YAML frontmatter.
fn parse_description(skill_md: &Path) -> Result<String> {
    let content = std::fs::read_to_string(skill_md)?;
    let content = content.trim();
    if !content.starts_with("---") {
        bail!("No frontmatter found");
    }
    let end = content[3..].find("---").context("Unclosed frontmatter")?;
    let frontmatter = &content[3..3 + end];
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(desc) = line.strip_prefix("description:") {
            return Ok(desc.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    bail!("No description field in frontmatter")
}

/// Copy a directory recursively.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_registry() -> (TempDir, Registry) {
        let tmp = TempDir::new().unwrap();
        let dirs = AppDirs::new(tmp.path().to_path_buf());
        dirs.ensure_dirs().unwrap();
        (tmp, Registry::new(dirs))
    }

    #[test]
    fn test_create_and_list_skill() {
        let (_tmp, reg) = setup_test_registry();
        reg.create("my-skill", "A test skill").unwrap();
        let skills = reg.list().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].description.as_deref(), Some("A test skill"));
    }

    #[test]
    fn test_create_duplicate_fails() {
        let (_tmp, reg) = setup_test_registry();
        reg.create("my-skill", "First").unwrap();
        assert!(reg.create("my-skill", "Second").is_err());
    }

    #[test]
    fn test_remove_skill() {
        let (_tmp, reg) = setup_test_registry();
        reg.create("my-skill", "A test skill").unwrap();
        assert!(reg.exists("my-skill"));
        reg.remove("my-skill").unwrap();
        assert!(!reg.exists("my-skill"));
    }

    #[test]
    fn test_remove_nonexistent_fails() {
        let (_tmp, reg) = setup_test_registry();
        assert!(reg.remove("nope").is_err());
    }

    #[test]
    fn test_get_skill() {
        let (_tmp, reg) = setup_test_registry();
        reg.create("my-skill", "desc").unwrap();
        let skill = reg.get("my-skill").unwrap().unwrap();
        assert_eq!(skill.name, "my-skill");
        assert!(skill.files.contains(&"SKILL.md".to_string()));
    }

    #[test]
    fn test_tree_hash_deterministic() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("test-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: test\n---\nContent").unwrap();
        std::fs::write(dir.join("extra.md"), "Extra content").unwrap();

        let hash1 = compute_tree_hash(&dir).unwrap();
        let hash2 = compute_tree_hash(&dir).unwrap();
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
    }

    #[test]
    fn test_tree_hash_changes_on_content_change() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("test-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "Version 1").unwrap();
        let hash1 = compute_tree_hash(&dir).unwrap();

        std::fs::write(dir.join("SKILL.md"), "Version 2").unwrap();
        let hash2 = compute_tree_hash(&dir).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_add_from_local() {
        let tmp_src = TempDir::new().unwrap();
        let skill_dir = tmp_src.path().join("imported-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: imported-skill\ndescription: Imported\n---\nContent").unwrap();

        let (_tmp, reg) = setup_test_registry();
        let name = reg.add_from_local(&skill_dir).unwrap();
        assert_eq!(name, "imported-skill");
        assert!(reg.exists("imported-skill"));
    }

    #[test]
    fn test_copy_dir_recursive() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "hello").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "world").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(std::fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(), "world");
    }
}
```

- [ ] **Step 2: Update lib.rs**

```rust
pub mod config;
pub mod db;
pub mod registry;

pub use config::{
    AgentsConfig, AppDirs, ProfilesConfig, SourcesConfig,
};
pub use db::Database;
pub use registry::Registry;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p skills-core`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/skills-core/
git commit -m "feat(core): add registry module with skill CRUD and tree hashing"
```

---

## Chunk 2: Profile Engine + Placement Engine

### Task 5: Profile Resolution Engine

**Files:**
- Create: `crates/skills-core/src/profiles.rs`
- Modify: `crates/skills-core/src/lib.rs`

The profile engine resolves composable profiles into a flat list of skill names, handling transitive expansion, circular detection, and deduplication.

- [ ] **Step 1: Write profile resolution module with tests**

Create `crates/skills-core/src/profiles.rs`:
```rust
use anyhow::{bail, Result};
use std::collections::{BTreeSet, HashSet};

use crate::config::ProfilesConfig;

/// Resolve a profile name into a deduplicated set of skill names,
/// including transitive includes and base skills.
pub fn resolve_profile(
    config: &ProfilesConfig,
    profile_name: &str,
    include_base: bool,
) -> Result<Vec<String>> {
    let mut skills = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut path = Vec::new();

    resolve_recursive(config, profile_name, &mut skills, &mut visited, &mut path)?;

    if include_base {
        for skill in &config.base.skills {
            skills.insert(skill.clone());
        }
    }

    Ok(skills.into_iter().collect())
}

/// Resolve base skills only.
pub fn resolve_base(config: &ProfilesConfig) -> Vec<String> {
    config.base.skills.clone()
}

/// Resolve multiple active profiles + base into a flat deduplicated skill list.
pub fn resolve_active_profiles(
    config: &ProfilesConfig,
    active_profiles: &[String],
) -> Result<Vec<String>> {
    let mut skills = BTreeSet::new();

    // Base skills always included
    for skill in &config.base.skills {
        skills.insert(skill.clone());
    }

    // Each active profile
    for profile_name in active_profiles {
        let profile_skills = resolve_profile(config, profile_name, false)?;
        for skill in profile_skills {
            skills.insert(skill);
        }
    }

    Ok(skills.into_iter().collect())
}

fn resolve_recursive(
    config: &ProfilesConfig,
    profile_name: &str,
    skills: &mut BTreeSet<String>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Result<()> {
    if !visited.insert(profile_name.to_string()) {
        // Circular include detected
        path.push(profile_name.to_string());
        let cycle = path.join(" -> ");
        bail!("Circular include detected: {}", cycle);
    }
    path.push(profile_name.to_string());

    let profile = config
        .profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;

    // Add this profile's own skills
    for skill in &profile.skills {
        skills.insert(skill.clone());
    }

    // Recursively resolve includes
    for included in &profile.includes {
        resolve_recursive(config, included, skills, visited, path)?;
    }

    path.pop();
    Ok(())
}

/// Validate that a profile config has no circular includes.
/// Returns Ok(()) if valid, Err with the cycle description if not.
pub fn validate_no_cycles(config: &ProfilesConfig) -> Result<()> {
    for profile_name in config.profiles.keys() {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        resolve_recursive(config, profile_name, &mut BTreeSet::new(), &mut visited, &mut path)?;
    }
    Ok(())
}

/// Validate that all skill names referenced in profiles exist in the registry.
pub fn validate_skills_exist(config: &ProfilesConfig, registry_skills: &[String]) -> Vec<String> {
    let known: HashSet<&str> = registry_skills.iter().map(|s| s.as_str()).collect();
    let mut missing = Vec::new();

    for skill in &config.base.skills {
        if !known.contains(skill.as_str()) {
            missing.push(skill.clone());
        }
    }

    for profile in config.profiles.values() {
        for skill in &profile.skills {
            if !known.contains(skill.as_str()) && !missing.contains(skill) {
                missing.push(skill.clone());
            }
        }
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BaseConfig, ProfileDef};
    use std::collections::BTreeMap;

    fn make_config() -> ProfilesConfig {
        let mut profiles = BTreeMap::new();
        profiles.insert("rust".into(), ProfileDef {
            description: Some("Rust".into()),
            skills: vec!["rust-engineer".into(), "cargo-patterns".into()],
            includes: vec![],
        });
        profiles.insert("react".into(), ProfileDef {
            description: Some("React".into()),
            skills: vec!["react-specialist".into()],
            includes: vec![],
        });
        profiles.insert("rust-react".into(), ProfileDef {
            description: Some("Full-stack".into()),
            skills: vec!["api-design".into()],
            includes: vec!["rust".into(), "react".into()],
        });

        ProfilesConfig {
            base: BaseConfig { skills: vec!["code-review".into(), "obsidian".into()] },
            profiles,
        }
    }

    #[test]
    fn test_resolve_simple_profile() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust", true).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(skills.contains(&"cargo-patterns".to_string()));
        assert!(skills.contains(&"code-review".to_string())); // base included
    }

    #[test]
    fn test_resolve_without_base() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust", false).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(!skills.contains(&"code-review".to_string())); // base excluded
    }

    #[test]
    fn test_resolve_composite_profile() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust-react", true).unwrap();
        assert!(skills.contains(&"api-design".to_string()));
        assert!(skills.contains(&"rust-engineer".to_string())); // from rust include
        assert!(skills.contains(&"react-specialist".to_string())); // from react include
        assert!(skills.contains(&"code-review".to_string())); // base
    }

    #[test]
    fn test_resolve_deduplicates() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust-react", true).unwrap();
        let unique: HashSet<&String> = skills.iter().collect();
        assert_eq!(skills.len(), unique.len());
    }

    #[test]
    fn test_circular_include_detected() {
        let mut profiles = BTreeMap::new();
        profiles.insert("a".into(), ProfileDef {
            description: None,
            skills: vec!["s1".into()],
            includes: vec!["b".into()],
        });
        profiles.insert("b".into(), ProfileDef {
            description: None,
            skills: vec!["s2".into()],
            includes: vec!["a".into()],
        });
        let config = ProfilesConfig {
            base: BaseConfig::default(),
            profiles,
        };
        let result = resolve_profile(&config, "a", false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular include detected"));
    }

    #[test]
    fn test_nonexistent_profile_fails() {
        let config = make_config();
        assert!(resolve_profile(&config, "nonexistent", false).is_err());
    }

    #[test]
    fn test_resolve_active_profiles() {
        let config = make_config();
        let skills = resolve_active_profiles(&config, &["rust".into(), "react".into()]).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(skills.contains(&"react-specialist".to_string()));
        assert!(skills.contains(&"code-review".to_string()));
    }

    #[test]
    fn test_validate_no_cycles_ok() {
        let config = make_config();
        assert!(validate_no_cycles(&config).is_ok());
    }

    #[test]
    fn test_validate_skills_exist() {
        let config = make_config();
        let registry = vec!["code-review".into(), "obsidian".into(), "rust-engineer".into()];
        let missing = validate_skills_exist(&config, &registry);
        assert!(missing.contains(&"cargo-patterns".to_string()));
        assert!(missing.contains(&"react-specialist".to_string()));
        assert!(!missing.contains(&"rust-engineer".to_string()));
    }

    #[test]
    fn test_transitive_three_levels() {
        let mut profiles = BTreeMap::new();
        profiles.insert("c".into(), ProfileDef {
            description: None,
            skills: vec!["s3".into()],
            includes: vec![],
        });
        profiles.insert("b".into(), ProfileDef {
            description: None,
            skills: vec!["s2".into()],
            includes: vec!["c".into()],
        });
        profiles.insert("a".into(), ProfileDef {
            description: None,
            skills: vec!["s1".into()],
            includes: vec!["b".into()],
        });
        let config = ProfilesConfig { base: BaseConfig::default(), profiles };
        let skills = resolve_profile(&config, "a", false).unwrap();
        assert!(skills.contains(&"s1".to_string()));
        assert!(skills.contains(&"s2".to_string()));
        assert!(skills.contains(&"s3".to_string()));
    }
}
```

- [ ] **Step 2: Update lib.rs and run tests**

Add `pub mod profiles;` to lib.rs.

Run: `cargo test -p skills-core -- profiles`
Expected: All 9 profile tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/skills-core/
git commit -m "feat(core): add profile resolution with transitive expansion and cycle detection"
```

---

### Task 6: Placement Engine

**Files:**
- Create: `crates/skills-core/src/placements.rs`
- Modify: `crates/skills-core/src/db.rs` (add placement queries)
- Modify: `crates/skills-core/src/lib.rs`

The placement engine handles activate/deactivate/switch — the core workflow.

- [ ] **Step 1: Add placement DB queries to db.rs**

Add these methods to the `Database` impl in `db.rs`:

```rust
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

    pub async fn get_active_profiles(&self, project_id: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT profile_name FROM project_profiles WHERE project_id = ? ORDER BY activated_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }
```

Add the PlacementRow struct:
```rust
#[derive(Debug, sqlx::FromRow)]
pub struct PlacementRow {
    pub id: i64,
    pub project_id: i64,
    pub skill_name: String,
    pub agent_name: String,
    pub target_path: String,
    pub placed_at: String,
}
```

- [ ] **Step 2: Write placement engine module**

Create `crates/skills-core/src/placements.rs`:
```rust
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::config::{AgentsConfig, AppDirs, ProfilesConfig};
use crate::db::Database;
use crate::profiles;
use crate::registry;

/// Result of an activation operation.
#[derive(Debug)]
pub struct ActivationResult {
    pub profile_name: String,
    pub skills_placed: usize,
    pub agents_used: Vec<String>,
    pub total_placements: usize,
}

/// Result of a deactivation operation.
#[derive(Debug)]
pub struct DeactivationResult {
    pub profile_name: String,
    pub files_removed: usize,
    pub files_kept: usize,
}

/// Activate a profile for a project.
///
/// 1. Resolve skills (profile + base)
/// 2. For each skill x agent, compute target path
/// 3. Check conflicts
/// 4. Copy skill directories atomically
/// 5. Record placements in DB
pub async fn activate(
    dirs: &AppDirs,
    db: &Database,
    profiles_config: &ProfilesConfig,
    agents_config: &AgentsConfig,
    profile_name: &str,
    project_path: &str,
    force: bool,
) -> Result<ActivationResult> {
    // Resolve skills
    let skills = profiles::resolve_profile(profiles_config, profile_name, true)?;

    // Get or create project
    let project_name = Path::new(project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let project_id = db.get_or_create_project(project_path, project_name.as_deref()).await?;

    // Determine which agents to place into
    let agents: Vec<(String, String)> = agents_config
        .agents
        .iter()
        .map(|(name, def)| (name.clone(), def.project_path.clone()))
        .collect();

    if agents.is_empty() {
        bail!("No agents configured. Run `skills-mgr agent add <name>` first.");
    }

    // Compute all placements and check conflicts
    struct PlannedPlacement {
        skill_name: String,
        agent_name: String,
        target_path: String,
    }

    let mut planned: Vec<PlannedPlacement> = Vec::new();
    for skill_name in &skills {
        // Check skill exists in registry
        if !dirs.registry().join(skill_name).join("SKILL.md").exists() {
            bail!("Skill '{}' not found in registry", skill_name);
        }

        for (agent_name, agent_project_path) in &agents {
            let target = PathBuf::from(project_path)
                .join(agent_project_path)
                .join(skill_name);
            let target_str = target.to_string_lossy().to_string();

            // Check for conflicts
            if let Some(existing) = db.find_conflict(project_id, &target_str).await? {
                if existing.skill_name == *skill_name {
                    // Same skill — just add profile link, no conflict
                } else if force {
                    // Different skill at same path — force overwrite
                    db.delete_placement(existing.id).await?;
                    if target.exists() {
                        std::fs::remove_dir_all(&target)?;
                    }
                } else {
                    bail!(
                        "Conflict: target path {} already occupied by skill '{}'. Use --force to overwrite.",
                        target_str,
                        existing.skill_name
                    );
                }
            }

            planned.push(PlannedPlacement {
                skill_name: skill_name.clone(),
                agent_name: agent_name.clone(),
                target_path: target_str,
            });
        }
    }

    // Execute placements atomically
    let mut placed_paths: Vec<PathBuf> = Vec::new();
    for p in &planned {
        let src = dirs.registry().join(&p.skill_name);
        let dst = PathBuf::from(&p.target_path);

        // Skip if already placed (same skill deduplicated)
        if dst.exists() {
            // Check if it's the same skill by seeing if we already placed it
            if let Some(existing) = db.find_conflict(project_id, &p.target_path).await? {
                if existing.skill_name == p.skill_name {
                    // Just link the profile, don't re-copy
                    let pid = existing.id;
                    db.link_placement_profile(pid, profile_name).await?;
                    continue;
                }
            }
        }

        if let Err(e) = registry::copy_dir_recursive(&src, &dst) {
            // Rollback all placed paths
            for rollback_path in &placed_paths {
                let _ = std::fs::remove_dir_all(rollback_path);
            }
            bail!("Failed to copy skill '{}' to '{}': {}. All placements rolled back.", p.skill_name, p.target_path, e);
        }
        placed_paths.push(dst);
    }

    // Record in DB
    for p in &planned {
        let placement_id = db.insert_placement(project_id, &p.skill_name, &p.agent_name, &p.target_path).await?;
        db.link_placement_profile(placement_id, profile_name).await?;
    }

    // Record active profile
    db.activate_project_profile(project_id, profile_name).await?;

    let agents_used: Vec<String> = agents.iter().map(|(n, _)| n.clone()).collect();
    Ok(ActivationResult {
        profile_name: profile_name.to_string(),
        skills_placed: skills.len(),
        agents_used,
        total_placements: planned.len(),
    })
}

/// Deactivate a profile for a project.
///
/// 1. Find placements linked to this profile
/// 2. Unlink profile from each placement
/// 3. Remove placements (and files) that have no remaining profile links
pub async fn deactivate(
    db: &Database,
    profile_name: &str,
    project_path: &str,
) -> Result<DeactivationResult> {
    let project_id = db.get_or_create_project(project_path, None).await?;

    let placements = db.get_placements_for_project_profile(project_id, profile_name).await?;

    let mut files_removed = 0;
    let mut files_kept = 0;

    for placement in &placements {
        db.unlink_placement_profile(placement.id, profile_name).await?;
        let remaining = db.get_placement_profile_count(placement.id).await?;

        if remaining == 0 {
            // No more profiles reference this placement — remove file and DB record
            let target = Path::new(&placement.target_path);
            if target.exists() {
                std::fs::remove_dir_all(target)?;
            }
            db.delete_placement(placement.id).await?;
            files_removed += 1;
        } else {
            files_kept += 1;
        }
    }

    db.deactivate_project_profile(project_id, profile_name).await?;

    Ok(DeactivationResult {
        profile_name: profile_name.to_string(),
        files_removed,
        files_kept,
    })
}

/// Get status for a project — active profiles and placement count.
pub async fn status(
    db: &Database,
    profiles_config: &ProfilesConfig,
    project_path: &str,
) -> Result<ProjectStatus> {
    let project_id = db.get_or_create_project(project_path, None).await?;
    let active_profiles = db.get_active_profiles(project_id).await?;
    let placements = db.get_all_placements_for_project(project_id).await?;

    Ok(ProjectStatus {
        project_path: project_path.to_string(),
        base_skills: profiles_config.base.skills.clone(),
        active_profiles,
        placement_count: placements.len(),
    })
}

#[derive(Debug)]
pub struct ProjectStatus {
    pub project_path: String,
    pub base_skills: Vec<String>,
    pub active_profiles: Vec<String>,
    pub placement_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, AppDirs, Database, ProfilesConfig, AgentsConfig) {
        let tmp = TempDir::new().unwrap();
        let dirs = AppDirs::new(tmp.path().join("skills-mgr"));
        dirs.ensure_dirs().unwrap();

        let db = Database::open_memory().await.unwrap();

        // Create test skills in registry
        for name in &["code-review", "rust-engineer", "react-specialist", "api-design"] {
            let skill_dir = dirs.registry().join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(
                skill_dir.join("SKILL.md"),
                format!("---\nname: {}\ndescription: Test skill\n---\nContent", name),
            ).unwrap();
        }

        let profiles_config = ProfilesConfig {
            base: BaseConfig { skills: vec!["code-review".into()] },
            profiles: {
                let mut m = BTreeMap::new();
                m.insert("rust".into(), ProfileDef {
                    description: Some("Rust".into()),
                    skills: vec!["rust-engineer".into()],
                    includes: vec![],
                });
                m.insert("react".into(), ProfileDef {
                    description: Some("React".into()),
                    skills: vec!["react-specialist".into()],
                    includes: vec![],
                });
                m
            },
        };

        let agents_config = AgentsConfig {
            agents: {
                let mut m = BTreeMap::new();
                m.insert("test-agent".into(), AgentDef {
                    project_path: ".test-agent/skills".into(),
                    global_path: "~/.test-agent/skills".into(),
                });
                m
            },
        };

        (tmp, dirs, db, profiles_config, agents_config)
    }

    #[tokio::test]
    async fn test_activate_profile() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        let result = activate(&dirs, &db, &profiles, &agents, "rust", &project_path.to_string_lossy(), false).await.unwrap();
        assert_eq!(result.profile_name, "rust");
        assert_eq!(result.skills_placed, 2); // code-review (base) + rust-engineer

        // Verify files were placed
        assert!(project_path.join(".test-agent/skills/rust-engineer/SKILL.md").exists());
        assert!(project_path.join(".test-agent/skills/code-review/SKILL.md").exists());
    }

    #[tokio::test]
    async fn test_deactivate_profile() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        activate(&dirs, &db, &profiles, &agents, "rust", &project_path.to_string_lossy(), false).await.unwrap();
        let result = deactivate(&db, "rust", &project_path.to_string_lossy()).await.unwrap();

        assert!(result.files_removed > 0);
        // Verify files were removed
        assert!(!project_path.join(".test-agent/skills/rust-engineer/SKILL.md").exists());
    }

    #[tokio::test]
    async fn test_composable_deactivation_keeps_shared() {
        let (tmp, dirs, db, mut profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        // Both rust and react profiles share base "code-review"
        activate(&dirs, &db, &profiles, &agents, "rust", &project_path.to_string_lossy(), false).await.unwrap();
        activate(&dirs, &db, &profiles, &agents, "react", &project_path.to_string_lossy(), false).await.unwrap();

        // Deactivate rust — code-review should remain (still used by react)
        let result = deactivate(&db, "rust", &project_path.to_string_lossy()).await.unwrap();
        assert!(result.files_kept > 0); // code-review kept
        assert!(project_path.join(".test-agent/skills/code-review/SKILL.md").exists());
        assert!(!project_path.join(".test-agent/skills/rust-engineer/SKILL.md").exists());
    }

    #[tokio::test]
    async fn test_status() {
        let (tmp, dirs, db, profiles, agents) = setup().await;
        let project_path = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_path).unwrap();

        activate(&dirs, &db, &profiles, &agents, "rust", &project_path.to_string_lossy(), false).await.unwrap();

        let s = status(&db, &profiles, &project_path.to_string_lossy()).await.unwrap();
        assert_eq!(s.active_profiles, vec!["rust"]);
        assert!(s.placement_count > 0);
    }
}
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod config;
pub mod db;
pub mod profiles;
pub mod placements;
pub mod registry;

pub use config::{AgentsConfig, AppDirs, ProfilesConfig, SourcesConfig};
pub use db::Database;
pub use registry::Registry;
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p skills-core`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/skills-core/
git commit -m "feat(core): add placement engine with activate/deactivate and composable profile support"
```

---

### Task 7: Operation Logging Integration

**Files:**
- Create: `crates/skills-core/src/logging.rs`
- Modify: `crates/skills-core/src/lib.rs`

Thin wrapper around `db.log_operation()` that provides a consistent API for CLI/MCP/GUI to log through.

- [ ] **Step 1: Write logging module**

Create `crates/skills-core/src/logging.rs`:
```rust
use anyhow::Result;
use crate::db::Database;

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Cli,
    Mcp,
    Gui,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Cli => "cli",
            Source::Mcp => "mcp",
            Source::Gui => "gui",
        }
    }
}

pub async fn log(
    db: &Database,
    source: Source,
    agent_name: Option<&str>,
    operation: &str,
    params: Option<&serde_json::Value>,
    project_path: Option<&str>,
    result: &str,
    details: &str,
) -> Result<()> {
    let params_str = params.map(|p| p.to_string());
    db.log_operation(
        source.as_str(),
        agent_name,
        operation,
        params_str.as_deref(),
        project_path,
        result,
        Some(details),
    ).await
}
```

- [ ] **Step 2: Update lib.rs, run tests, commit**

Add `pub mod logging;` to lib.rs.

Run: `cargo test -p skills-core`

```bash
git add crates/skills-core/
git commit -m "feat(core): add operation logging module"
```

---

## Chunk 3: CLI

### Task 8: CLI Command Structure

**Files:**
- Modify: `crates/skills-cli/src/main.rs`
- Create: `crates/skills-cli/src/commands/mod.rs`
- Create: `crates/skills-cli/src/commands/skill.rs`
- Create: `crates/skills-cli/src/commands/profile.rs`
- Create: `crates/skills-cli/src/commands/agent.rs`
- Create: `crates/skills-cli/src/commands/status.rs`
- Create: `crates/skills-cli/src/commands/util.rs`

This is a large task. The CLI is a thin wrapper over `skills-core`. Each subcommand module calls into the core library.

- [ ] **Step 1: Define CLI argument structure with clap**

Update `crates/skills-cli/src/main.rs`:
```rust
mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;
use skills_core::{AppDirs, Database};

#[derive(Parser)]
#[command(name = "skills-mgr", about = "Cross-agent skill management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage skills in the registry
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Manage agent configurations
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Show active profiles and placements for a project
    Status {
        /// Target project path (default: current directory)
        #[arg(long)]
        project: Option<String>,
    },
    /// Scan for overlapping skills across active profiles
    CheckConflicts {
        #[arg(long)]
        project: Option<String>,
    },
    /// Verify placements match DB, check for orphans
    Doctor,
    /// Estimate token cost of active or specified profile
    Budget {
        /// Profile name (default: current active set)
        profile: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Show recent operations
    Log {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value = "20")]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    List,
    Add { source: String },
    Remove { name: String },
    Update { name: Option<String>, #[arg(long)] all: bool },
    Info { name: String },
    Create { name: String, #[arg(long)] description: Option<String> },
    Open { name: String },
    Files { name: String },
}

#[derive(Subcommand)]
enum ProfileAction {
    List,
    Create {
        name: String,
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,
    },
    Delete { name: String },
    Show { name: String },
    Edit {
        name: String,
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        remove: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,
    },
    Activate {
        name: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        force: bool,
    },
    Deactivate {
        name: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        global: bool,
    },
    Switch {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum AgentAction {
    List,
    Add {
        name: String,
        #[arg(long)]
        project_path: String,
        #[arg(long)]
        global_path: String,
    },
    Remove { name: String },
    Enable {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
    Disable {
        name: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let base = AppDirs::default_base()?;
    let dirs = AppDirs::new(base);
    dirs.ensure_dirs()?;

    let db = Database::open(&dirs.database()).await?;

    match cli.command {
        Commands::Skill { action } => commands::skill::run(&dirs, &db, action).await?,
        Commands::Profile { action } => commands::profile::run(&dirs, &db, action).await?,
        Commands::Agent { action } => commands::agent::run(&dirs, &db, action).await?,
        Commands::Status { project } => commands::status::run(&dirs, &db, project).await?,
        Commands::Log { project, source, limit } => {
            commands::util::show_log(&db, limit).await?;
        }
        Commands::CheckConflicts { project } => {
            println!("check-conflicts: not yet implemented");
        }
        Commands::Doctor => {
            println!("doctor: not yet implemented");
        }
        Commands::Budget { profile, project } => {
            println!("budget: not yet implemented");
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Create command modules**

Create `crates/skills-cli/src/commands/mod.rs`:
```rust
pub mod skill;
pub mod profile;
pub mod agent;
pub mod status;
pub mod util;
```

Create `crates/skills-cli/src/commands/skill.rs`:
```rust
use anyhow::Result;
use skills_core::{AppDirs, Database, Registry};
use crate::SkillAction;

pub async fn run(dirs: &AppDirs, db: &Database, action: SkillAction) -> Result<()> {
    let registry = Registry::new(dirs.clone());

    match action {
        SkillAction::List => {
            let skills = registry.list()?;
            if skills.is_empty() {
                println!("No skills in registry. Run `skills-mgr skill add <source>` to add one.");
                return Ok(());
            }
            for skill in &skills {
                let desc = skill.description.as_deref().unwrap_or("(no description)");
                let source_type = skill.source.as_ref()
                    .map(|s| format!("{:?}", s.source_type).to_lowercase())
                    .unwrap_or("unknown".into());
                println!("  {} [{}] - {}", skill.name, source_type, desc);
            }
            println!("\n{} skills total", skills.len());
        }
        SkillAction::Info { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    println!("Name: {}", skill.name);
                    println!("Description: {}", skill.description.as_deref().unwrap_or("(none)"));
                    println!("Path: {}", skill.dir_path.display());
                    println!("Files:");
                    for f in &skill.files {
                        println!("  {}", f);
                    }
                    if let Some(src) = &skill.source {
                        println!("Source: {:?}", src.source_type);
                        if let Some(url) = &src.url { println!("URL: {}", url); }
                        if let Some(hash) = &src.hash { println!("Hash: {}", hash); }
                    }
                }
                None => println!("Skill '{}' not found in registry", name),
            }
        }
        SkillAction::Create { name, description } => {
            let desc = description.as_deref().unwrap_or("TODO: add description");
            let path = registry.create(&name, desc)?;
            println!("Created skill '{}' at {}", name, path.display());
        }
        SkillAction::Remove { name } => {
            registry.remove(&name)?;
            println!("Removed skill '{}' from registry", name);
        }
        SkillAction::Files { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    for f in &skill.files {
                        println!("  {}", f);
                    }
                }
                None => println!("Skill '{}' not found", name),
            }
        }
        SkillAction::Add { source } => {
            let path = std::path::Path::new(&source);
            if path.exists() {
                let name = registry.add_from_local(path)?;
                println!("Added skill '{}' from local path", name);
            } else {
                println!("Git-based skill import not yet implemented. Use a local path for now.");
            }
        }
        SkillAction::Update { name, all } => {
            println!("skill update: not yet implemented");
        }
        SkillAction::Open { name } => {
            match registry.get(&name)? {
                Some(skill) => {
                    open::that(&skill.dir_path)?;
                }
                None => println!("Skill '{}' not found", name),
            }
        }
    }

    Ok(())
}
```

Create `crates/skills-cli/src/commands/profile.rs`:
```rust
use anyhow::Result;
use skills_core::config::{ProfileDef, ProfilesConfig, AgentsConfig};
use skills_core::{AppDirs, Database};
use skills_core::placements;
use skills_core::profiles;
use crate::ProfileAction;

pub async fn run(dirs: &AppDirs, db: &Database, action: ProfileAction) -> Result<()> {
    let mut profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        ProfileAction::List => {
            println!("Base skills: {}", profiles_config.base.skills.join(", "));
            if profiles_config.profiles.is_empty() {
                println!("\nNo profiles defined.");
            } else {
                println!("\nProfiles:");
                for (name, profile) in &profiles_config.profiles {
                    let desc = profile.description.as_deref().unwrap_or("");
                    let skills = profile.skills.join(", ");
                    let includes = if profile.includes.is_empty() {
                        String::new()
                    } else {
                        format!(" (includes: {})", profile.includes.join(", "))
                    };
                    println!("  {} - {} [{}]{}", name, desc, skills, includes);
                }
            }
        }
        ProfileAction::Show { name } => {
            let resolved = profiles::resolve_profile(&profiles_config, &name, true)?;
            println!("Profile '{}' resolves to {} skills:", name, resolved.len());
            for skill in &resolved {
                println!("  {}", skill);
            }
        }
        ProfileAction::Create { name, add, include } => {
            let profile = ProfileDef {
                description: None,
                skills: add,
                includes: include,
            };
            profiles_config.profiles.insert(name.clone(), profile);
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Created profile '{}'", name);
        }
        ProfileAction::Delete { name } => {
            if profiles_config.profiles.remove(&name).is_some() {
                profiles_config.save(&dirs.profiles_toml())?;
                println!("Deleted profile '{}'", name);
            } else {
                println!("Profile '{}' not found", name);
            }
        }
        ProfileAction::Edit { name, add, remove, include } => {
            let profile = profiles_config.profiles.get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;
            for s in &add { if !profile.skills.contains(s) { profile.skills.push(s.clone()); } }
            profile.skills.retain(|s| !remove.contains(s));
            for i in &include { if !profile.includes.contains(i) { profile.includes.push(i.clone()); } }
            profiles::validate_no_cycles(&profiles_config)?;
            profiles_config.save(&dirs.profiles_toml())?;
            println!("Updated profile '{}'", name);
        }
        ProfileAction::Activate { name, project, global, force } => {
            let project_path = resolve_project_path(project)?;
            let result = placements::activate(dirs, db, &profiles_config, &agents_config, &name, &project_path, force).await?;
            println!("Activated profile '{}' for {}", result.profile_name, project_path);
            println!("  {} skills -> {} ({} placements)", result.skills_placed, result.agents_used.join(", "), result.total_placements);
        }
        ProfileAction::Deactivate { name, project, global } => {
            let project_path = resolve_project_path(project)?;
            let result = placements::deactivate(db, &name, &project_path).await?;
            println!("Deactivated profile '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept);
        }
        ProfileAction::Switch { name, project } => {
            let project_path = resolve_project_path(project)?;
            let project_id = db.get_or_create_project(&project_path, None).await?;
            let active = db.get_active_profiles(project_id).await?;

            // Deactivate all current profiles (except base)
            for p in &active {
                placements::deactivate(db, p, &project_path).await?;
            }

            // Activate new profile
            let result = placements::activate(dirs, db, &profiles_config, &agents_config, &name, &project_path, false).await?;
            println!("Switched to profile '{}' ({} placements)", result.profile_name, result.total_placements);
        }
    }

    Ok(())
}

fn resolve_project_path(project: Option<String>) -> Result<String> {
    match project {
        Some(p) => Ok(std::fs::canonicalize(&p)?.to_string_lossy().to_string()),
        None => Ok(std::env::current_dir()?.to_string_lossy().to_string()),
    }
}
```

Create `crates/skills-cli/src/commands/agent.rs`:
```rust
use anyhow::Result;
use skills_core::config::{AgentDef, AgentsConfig};
use skills_core::AppDirs;
use skills_core::Database;
use crate::AgentAction;

pub async fn run(dirs: &AppDirs, _db: &Database, action: AgentAction) -> Result<()> {
    let mut config = AgentsConfig::load(&dirs.agents_toml())?;

    match action {
        AgentAction::List => {
            if config.agents.is_empty() {
                println!("No agents configured. Run `skills-mgr agent add <name>` to add one.");
            } else {
                for (name, def) in &config.agents {
                    println!("  {} -> project: {}, global: {}", name, def.project_path, def.global_path);
                }
            }
        }
        AgentAction::Add { name, project_path, global_path } => {
            config.agents.insert(name.clone(), AgentDef { project_path, global_path });
            config.save(&dirs.agents_toml())?;
            println!("Added agent '{}'", name);
        }
        AgentAction::Remove { name } => {
            if config.agents.remove(&name).is_some() {
                config.save(&dirs.agents_toml())?;
                println!("Removed agent '{}'", name);
            } else {
                println!("Agent '{}' not found", name);
            }
        }
        AgentAction::Enable { name, project } => {
            println!("agent enable: not yet implemented");
        }
        AgentAction::Disable { name, project } => {
            println!("agent disable: not yet implemented");
        }
    }

    Ok(())
}
```

Create `crates/skills-cli/src/commands/status.rs`:
```rust
use anyhow::Result;
use skills_core::config::ProfilesConfig;
use skills_core::{AppDirs, Database};
use skills_core::placements;

pub async fn run(dirs: &AppDirs, db: &Database, project: Option<String>) -> Result<()> {
    let project_path = match project {
        Some(p) => std::fs::canonicalize(&p)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };

    let profiles_config = ProfilesConfig::load(&dirs.profiles_toml())?;
    let s = placements::status(db, &profiles_config, &project_path).await?;

    println!("Project: {}", s.project_path);
    println!("Base: [{}]", s.base_skills.join(", "));
    if s.active_profiles.is_empty() {
        println!("Active profiles: (none)");
    } else {
        println!("Active profiles: [{}]", s.active_profiles.join(", "));
    }
    println!("Placements: {}", s.placement_count);

    Ok(())
}
```

Create `crates/skills-cli/src/commands/util.rs`:
```rust
use anyhow::Result;
use skills_core::Database;

pub async fn show_log(db: &Database, limit: i64) -> Result<()> {
    let logs = db.get_recent_logs(limit).await?;
    if logs.is_empty() {
        println!("No operations logged yet.");
        return Ok(());
    }
    for entry in &logs {
        let agent = entry.agent_name.as_deref().unwrap_or("");
        let project = entry.project_path.as_deref().unwrap_or("");
        let details = entry.details.as_deref().unwrap_or("");
        println!(
            "{} | {} ({}) | {} | {} | {}",
            entry.timestamp, entry.operation, entry.source, agent, entry.result, details
        );
    }
    Ok(())
}
```

- [ ] **Step 3: Add `open` dependency to skills-cli**

Add to `crates/skills-cli/Cargo.toml`:
```toml
[dependencies]
skills-core = { path = "../skills-core" }
clap = { version = "4", features = ["derive"] }
anyhow.workspace = true
tokio.workspace = true
open = "5"
```

- [ ] **Step 4: Verify it compiles and runs**

Run: `cargo build -p skills-cli`
Run: `cargo run -p skills-cli -- --help`
Expected: Shows help text with all subcommands

Run: `cargo run -p skills-cli -- skill list`
Expected: "No skills in registry..." message

- [ ] **Step 5: Commit**

```bash
git add crates/skills-cli/
git commit -m "feat(cli): add full CLI command structure with skill, profile, agent, and status commands"
```

---

## Chunk 4: MCP Server

### Task 9: MCP Server Module

**Files:**
- Create: `crates/skills-mcp/Cargo.toml`
- Create: `crates/skills-mcp/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

The MCP server exposes skills-core operations as MCP tools. It's a library crate that will be embedded in the Tauri app.

- [ ] **Step 1: Add skills-mcp to workspace**

Add `"crates/skills-mcp"` to workspace members in root `Cargo.toml`.

Create `crates/skills-mcp/Cargo.toml`:
```toml
[package]
name = "skills-mcp"
version.workspace = true
edition.workspace = true

[dependencies]
skills-core = { path = "../skills-core" }
rmcp = { version = "0.16", features = ["server", "transport-sse-server"] }
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
tokio.workspace = true
schemars = "0.8"
```

- [ ] **Step 2: Write MCP server with tool definitions**

Create `crates/skills-mcp/src/lib.rs`:
```rust
use anyhow::Result;
use rmcp::prelude::*;
use skills_core::{AppDirs, Database, Registry};
use skills_core::config::{AgentsConfig, ProfilesConfig};
use std::sync::Arc;

/// MCP Server for skills-mgr.
/// Exposes skill and profile management tools.
pub struct SkillsMcpServer {
    dirs: AppDirs,
    db: Database,
}

impl SkillsMcpServer {
    pub fn new(dirs: AppDirs, db: Database) -> Self {
        Self { dirs, db }
    }
}

// Note: Full MCP tool implementation will follow the rmcp #[tool] macro pattern.
// Each CLI command maps to an MCP tool with the same parameters.
// This is a structural placeholder — the actual tool implementations call
// the same skills-core functions as the CLI.

// TODO: Implement MCP tools using rmcp macros once Tauri integration is set up.
// Each tool calls into skills-core the same way the CLI does.
// The tool definitions match the MCP Server Interface table in the design spec.
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p skills-mcp`
Expected: Compiles (placeholder only)

- [ ] **Step 4: Commit**

```bash
git add crates/skills-mcp/ Cargo.toml
git commit -m "feat(mcp): scaffold MCP server crate with rmcp dependency"
```

---

## Chunk 5: UI Design with Pencil

### Task 10: Design UI Mockups in Pencil ✅

**Files:**
- Created: `designs/skills-mgr-ui.pen` (Pencil design file)

**Design System:** Inter font, Lucide icons, OLED dark mode + light mode. Color tokens defined with semantic variables.

**Completed Designs (16 frames × 2 modes = 32 total):**

| # | Screen/Overlay | Dark ID | Light ID |
|---|---|---|---|
| 1 | Dashboard | `Zs4Kt` | `Ko7Cb` |
| 2 | Skills Registry | `sH6KO` | `KaoK5` |
| 3 | Profiles | `uhwyy` | `g8uTy` |
| 4 | Agents | `hnpP2` | `evaE1` |
| 5 | Settings | `7oGJ3` | `r52jA` |
| 6 | Activity Log | `OAwYJ` | `wz6Tt` |
| 7 | Add Skill Dialog | `yVxPk` | `50qyk` |
| 8 | Create Profile Dialog | `l4ZGj` | `uSulM` |
| 9 | Confirm Delete Dialog | `vIeIS` | `EC8id` |
| 10 | Skill Detail Panel | `CvLG1` | `s6a0D` |
| 11 | Toast Notifications | `ZL4WO` | `t1ltC` |
| 12 | Import Skills Dialog | `2k8SI` | `ggKXx` |
| 13 | Edit Profile Dialog | `C2rlh` | `HlM7J` |
| 14 | Skill Detail Multi-File | `LEarj` | `5JfdW` |
| 15 | Add Agent Dialog | `lRepD` | `5jTYH` |
| 16 | Edit Agent Dialog | `jAVYo` | `89xhq` |

**Reusable Components:** NavItem (default/active), StatCard, Badge, Button (primary/secondary)

- [x] **Step 1: Get Pencil guidelines and style guide**
- [x] **Step 2: Open a new Pencil document**
- [x] **Step 3: Design the App Shell / Layout**
- [x] **Step 4: Design the Dashboard page**
- [x] **Step 5: Design the Skills page**
- [x] **Step 6: Design the Profiles page**
- [x] **Step 7: Design the Agents page**
- [x] **Step 8: Design the Settings page**
- [x] **Step 9: Design the Log Viewer page**
- [x] **Step 10: Design overlay UI (dialogs, panels, toasts) in dark + light mode**
- [x] **Step 11: Take screenshots and validate**
- [ ] **Step 12: Commit**

```bash
git add designs/
git commit -m "docs: add UI mockups designed with Pencil for all GUI views"
```

---

## Chunk 6: GUI Foundation

### Task 11: Tauri 2 + React Project Setup

**Files:**
- Create: `crates/skills-gui/` (entire Tauri + React scaffold)
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Install Tauri CLI**

Run: `cargo install tauri-cli --version "^2"`

- [ ] **Step 2: Scaffold Tauri app inside crates/skills-gui**

Run from project root:
```bash
cd crates && pnpm create tauri-app skills-gui --template react-ts --manager pnpm --yes
```

This creates the full Tauri 2 + React + TypeScript scaffold.

- [ ] **Step 3: Add skills-gui to workspace**

Add `"crates/skills-gui/src-tauri"` to workspace members in root `Cargo.toml`.

Add workspace dependencies to `crates/skills-gui/src-tauri/Cargo.toml`:
```toml
[dependencies]
skills-core = { path = "../../skills-core" }
skills-mcp = { path = "../../skills-mcp" }
```

- [ ] **Step 4: Install frontend dependencies**

```bash
cd crates/skills-gui && pnpm add @tanstack/react-query react-router sonner zod
cd crates/skills-gui && pnpm add -D tailwindcss @tailwindcss/vite
```

- [ ] **Step 5: Initialize shadcn/ui**

```bash
cd crates/skills-gui && pnpm dlx shadcn@latest init
```

Follow prompts: TypeScript, Tailwind CSS, default style.

- [ ] **Step 6: Add commonly needed shadcn components**

```bash
cd crates/skills-gui && pnpm dlx shadcn@latest add button card input table badge dialog tabs toast sidebar command
```

- [ ] **Step 7: Verify it builds**

Run: `cd crates/skills-gui && pnpm build`
Run: `cd crates/skills-gui && cargo tauri build --debug` (or `cargo tauri dev` to test)

- [ ] **Step 8: Commit**

```bash
git add crates/skills-gui/ Cargo.toml
git commit -m "feat(gui): scaffold Tauri 2 + React + shadcn/ui with dependencies"
```

---

### Task 12: Tauri IPC Commands

**Files:**
- Modify: `crates/skills-gui/src-tauri/src/main.rs`
- Create: `crates/skills-gui/src-tauri/src/commands.rs`

Wire up Tauri IPC so the React frontend can call skills-core functions.

- [ ] **Step 1: Create Tauri commands**

Create `crates/skills-gui/src-tauri/src/commands.rs`:
```rust
use skills_core::{AppDirs, Database, Registry};
use skills_core::config::{AgentsConfig, ProfilesConfig};
use skills_core::placements;
use tauri::State;
use serde::Serialize;

pub struct AppState {
    pub dirs: AppDirs,
    pub db: Database,
}

#[derive(Serialize)]
pub struct SkillInfo {
    name: String,
    description: Option<String>,
    files: Vec<String>,
    source_type: Option<String>,
}

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillInfo>, String> {
    let registry = Registry::new(state.dirs.clone());
    let skills = registry.list().map_err(|e| e.to_string())?;
    Ok(skills.into_iter().map(|s| SkillInfo {
        name: s.name,
        description: s.description,
        files: s.files,
        source_type: s.source.map(|src| format!("{:?}", src.source_type).to_lowercase()),
    }).collect())
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>, project_path: String) -> Result<serde_json::Value, String> {
    let profiles_config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let s = placements::status(&state.db, &profiles_config, &project_path).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "project_path": s.project_path,
        "base_skills": s.base_skills,
        "active_profiles": s.active_profiles,
        "placement_count": s.placement_count,
    }))
}

#[tauri::command]
pub async fn activate_profile(
    state: State<'_, AppState>,
    profile_name: String,
    project_path: String,
    force: bool,
) -> Result<String, String> {
    let profiles_config = ProfilesConfig::load(&state.dirs.profiles_toml()).map_err(|e| e.to_string())?;
    let agents_config = AgentsConfig::load(&state.dirs.agents_toml()).map_err(|e| e.to_string())?;
    let result = placements::activate(&state.dirs, &state.db, &profiles_config, &agents_config, &profile_name, &project_path, force)
        .await.map_err(|e| e.to_string())?;
    Ok(format!("Activated '{}': {} skills, {} placements", result.profile_name, result.skills_placed, result.total_placements))
}

#[tauri::command]
pub async fn deactivate_profile(
    state: State<'_, AppState>,
    profile_name: String,
    project_path: String,
) -> Result<String, String> {
    let result = placements::deactivate(&state.db, &profile_name, &project_path)
        .await.map_err(|e| e.to_string())?;
    Ok(format!("Deactivated '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept))
}

#[tauri::command]
pub async fn get_recent_logs(state: State<'_, AppState>, limit: i64) -> Result<serde_json::Value, String> {
    let logs = state.db.get_recent_logs(limit).await.map_err(|e| e.to_string())?;
    let entries: Vec<serde_json::Value> = logs.into_iter().map(|l| serde_json::json!({
        "id": l.id,
        "timestamp": l.timestamp,
        "source": l.source,
        "agent_name": l.agent_name,
        "operation": l.operation,
        "result": l.result,
        "details": l.details,
    })).collect();
    Ok(serde_json::Value::Array(entries))
}
```

- [ ] **Step 2: Update Tauri main.rs to register commands**

Update `crates/skills-gui/src-tauri/src/main.rs`:
```rust
mod commands;

use commands::AppState;
use skills_core::{AppDirs, Database};

#[tokio::main]
async fn main() {
    let base = AppDirs::default_base().expect("Failed to determine home directory");
    let dirs = AppDirs::new(base);
    dirs.ensure_dirs().expect("Failed to create app directories");

    let db = Database::open(&dirs.database())
        .await
        .expect("Failed to open database");

    tauri::Builder::default()
        .manage(AppState { dirs, db })
        .invoke_handler(tauri::generate_handler![
            commands::list_skills,
            commands::list_profiles,
            commands::list_agents,
            commands::get_status,
            commands::activate_profile,
            commands::deactivate_profile,
            commands::get_recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("Error running tauri application");
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd crates/skills-gui && cargo tauri build --debug`

- [ ] **Step 4: Commit**

```bash
git add crates/skills-gui/src-tauri/
git commit -m "feat(gui): add Tauri IPC commands wired to skills-core"
```

---

## Chunk 7: GUI Views + Polish

### Task 13: React App Shell + Routing

> **Design Reference:** `designs/skills-mgr-ui.pen` — Open with the Pencil MCP tool (`get_editor_state`, `batch_get`, `get_screenshot`). Contains 32 complete mockups (16 screens/overlays × dark + light mode) with exact colors, spacing, typography, and component structure. Use `batch_get` with frame IDs from the Task 10 table to inspect node properties (colors, padding, font sizes, layout). Implement each page to match the corresponding Pencil frame. Support both dark and light themes using the color tokens from the design.

**Files:**
- Modify: `crates/skills-gui/src/App.tsx`
- Create: `crates/skills-gui/src/layouts/AppLayout.tsx`
- Create: `crates/skills-gui/src/pages/Dashboard.tsx`
- Create: `crates/skills-gui/src/pages/Skills.tsx`
- Create: `crates/skills-gui/src/pages/Profiles.tsx`
- Create: `crates/skills-gui/src/pages/Projects.tsx`
- Create: `crates/skills-gui/src/pages/Agents.tsx`
- Create: `crates/skills-gui/src/pages/ActivityLog.tsx`
- Create: `crates/skills-gui/src/pages/Settings.tsx`
- Create: `crates/skills-gui/src/lib/api.ts`
- Create: `crates/skills-gui/src/lib/schemas.ts`

- [ ] **Step 1: Create Zod schemas**

Create `crates/skills-gui/src/lib/schemas.ts`:
```typescript
import { z } from "zod"

export const SkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  files: z.array(z.string()),
  source_type: z.string().nullable(),
})

export const LogEntrySchema = z.object({
  id: z.number(),
  timestamp: z.string(),
  source: z.string(),
  agent_name: z.string().nullable(),
  operation: z.string(),
  result: z.string(),
  details: z.string().nullable(),
})

export type Skill = z.infer<typeof SkillSchema>
export type LogEntry = z.infer<typeof LogEntrySchema>
```

- [ ] **Step 2: Create Tauri API wrapper**

Create `crates/skills-gui/src/lib/api.ts`:
```typescript
import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import { SkillSchema, LogEntrySchema } from "./schemas"

export async function listSkills() {
  const data = await invoke("list_skills")
  return z.array(SkillSchema).parse(data)
}

export async function listProfiles() {
  return await invoke("list_profiles")
}

export async function listAgents() {
  return await invoke("list_agents")
}

export async function getStatus(projectPath: string) {
  return await invoke("get_status", { projectPath })
}

export async function activateProfile(profileName: string, projectPath: string, force = false) {
  return await invoke("activate_profile", { profileName, projectPath, force })
}

export async function deactivateProfile(profileName: string, projectPath: string) {
  return await invoke("deactivate_profile", { profileName, projectPath })
}

export async function getRecentLogs(limit = 20) {
  const data = await invoke("get_recent_logs", { limit })
  return z.array(LogEntrySchema).parse(data)
}
```

- [ ] **Step 3: Create app layout with sidebar**

> **Pencil ref:** Sidebar visible in all 6 main screens (frames 1–6). Inspect dark frame `Zs4Kt` for sidebar structure: 220px wide, `#111114` background, Lucide icons + Inter 14px labels. Active nav item uses `#6366F1` accent with `#6366F115` background. Logo area at top ("Skills Manager" + grid icon). Reusable components: NavItem/Default `hSOCH`, NavItem/Active `njj8S`.

Create `crates/skills-gui/src/layouts/AppLayout.tsx`:
```tsx
import { Link, Outlet, useLocation } from "react-router-dom"

const navItems = [
  { path: "/", label: "Dashboard", icon: "LayoutDashboard" },
  { path: "/skills", label: "Skills", icon: "Wrench" },
  { path: "/profiles", label: "Profiles", icon: "Layers" },
  { path: "/agents", label: "Agents", icon: "Bot" },
  { path: "/activity", label: "Activity", icon: "Activity" },
  { path: "/settings", label: "Settings", icon: "Settings" },
]

export function AppLayout() {
  const location = useLocation()

  return (
    <div className="flex h-screen bg-background">
      <nav className="w-56 border-r bg-muted/40 p-4">
        <h1 className="mb-6 text-lg font-semibold">Skills Manager</h1>
        <ul className="space-y-1">
          {navItems.map((item) => (
            <li key={item.path}>
              <Link
                to={item.path}
                className={`block rounded-md px-3 py-2 text-sm ${
                  location.pathname === item.path
                    ? "bg-primary text-primary-foreground"
                    : "hover:bg-muted"
                }`}
              >
                {item.label}
              </Link>
            </li>
          ))}
        </ul>
      </nav>
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  )
}
```

- [ ] **Step 4: Create page stubs**

> **Pencil refs per page:**
> - **Dashboard** → dark `Zs4Kt` / light `Ko7Cb` — 4 stat cards (grid), recent activity table, active profiles pills
> - **Skills** → dark `sH6KO` / light `KaoK5` — card grid with search bar, "⋮" overflow menus on cards; dialogs: Add Skill `yVxPk`, Import `2k8SI`, Detail Panel `CvLG1`/`LEarj`
> - **Profiles** → dark `uhwyy` / light `g8uTy` — table rows with "⋮" menus; dialogs: Create `l4ZGj`, Edit `C2rlh` (both have "Compose from Profiles" section)
> - **Agents** → dark `hnpP2` / light `evaE1` — table with status badges + "⋮" menus; dialogs: Add `lRepD`, Edit `jAVYo`
> - **Activity Log** → dark `OAwYJ` / light `wz6Tt` — filterable table with color-coded result column
> - **Settings** → dark `7oGJ3` / light `r52jA` — sectioned form (General, MCP Server, Git Sync, About)
> - **Shared overlays** → Delete Confirm `vIeIS`, Toasts `ZL4WO` (success/error/info)

Create each page file with a minimal placeholder. Example for `crates/skills-gui/src/pages/Dashboard.tsx`:
```tsx
import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listSkills, listProfiles, getRecentLogs } from "@/lib/api"

export function Dashboard() {
  const skills = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const logs = useQuery({ queryKey: ["logs"], queryFn: () => getRecentLogs(5) })

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Dashboard</h2>
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader><CardTitle>Skills</CardTitle></CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{skills.data?.length ?? 0}</p>
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader><CardTitle>Recent Activity</CardTitle></CardHeader>
        <CardContent>
          {logs.data?.map((log) => (
            <div key={log.id} className="flex justify-between border-b py-2 text-sm">
              <span>{log.operation} ({log.source})</span>
              <span className="text-muted-foreground">{log.timestamp}</span>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  )
}
```

Create similar stubs for Skills, Profiles, Projects, Agents, ActivityLog, Settings pages — each showing basic data from the API with shadcn/ui components.

- [ ] **Step 5: Wire up routing in App.tsx**

```tsx
import { BrowserRouter, Routes, Route } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { Toaster } from "sonner"
import { AppLayout } from "./layouts/AppLayout"
import { Dashboard } from "./pages/Dashboard"
import { Skills } from "./pages/Skills"
import { Profiles } from "./pages/Profiles"
import { Projects } from "./pages/Projects"
import { Agents } from "./pages/Agents"
import { ActivityLog } from "./pages/ActivityLog"
import { Settings } from "./pages/Settings"

const queryClient = new QueryClient()

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route path="/" element={<Dashboard />} />
            <Route path="/skills" element={<Skills />} />
            <Route path="/profiles" element={<Profiles />} />
            <Route path="/projects" element={<Projects />} />
            <Route path="/agents" element={<Agents />} />
            <Route path="/activity" element={<ActivityLog />} />
            <Route path="/settings" element={<Settings />} />
          </Route>
        </Routes>
      </BrowserRouter>
      <Toaster />
    </QueryClientProvider>
  )
}

export default App
```

- [ ] **Step 6: Verify GUI builds and runs**

Run: `cd crates/skills-gui && cargo tauri dev`
Expected: App opens with sidebar navigation. Dashboard shows skill count and recent activity.

- [ ] **Step 7: Commit**

```bash
git add crates/skills-gui/src/
git commit -m "feat(gui): add app shell with sidebar navigation, routing, and page stubs"
```

---

### Task 14: Teaching Skill

**Files:**
- Create: teaching skill in registry (first-run setup)

- [ ] **Step 1: Create the skills-mgr-guide skill**

This happens at first run or via `skills-mgr init`. For now, add the skill content to `crates/skills-core/src/lib.rs` as an embedded constant that gets written to the registry on first setup.

Add to `crates/skills-core/src/config.rs`:
```rust
/// Write the built-in teaching skill to the registry if not already present.
pub fn ensure_teaching_skill(dirs: &AppDirs) -> Result<()> {
    let skill_dir = dirs.registry().join("skills-mgr-guide");
    if skill_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(skill_dir.join("SKILL.md"), include_str!("../assets/skills-mgr-guide.md"))?;
    Ok(())
}
```

Create `crates/skills-core/assets/skills-mgr-guide.md` with the teaching skill content from the design spec (the full SKILL.md content from the Teaching Skill section).

- [ ] **Step 2: Call ensure_teaching_skill from AppDirs::ensure_dirs**

Add `ensure_teaching_skill(self)?;` at the end of `AppDirs::ensure_dirs()`.

- [ ] **Step 3: Run tests, commit**

Run: `cargo test -p skills-core`

```bash
git add crates/skills-core/
git commit -m "feat(core): embed and auto-install skills-mgr-guide teaching skill"
```

---

### Task 15: End-to-End Smoke Test

**Files:**
- No new files — this is a manual verification task

- [ ] **Step 1: Build the CLI**

Run: `cargo build -p skills-cli`

- [ ] **Step 2: Run the full workflow**

```bash
# Setup agents
./target/debug/skills-mgr agent add claude-code --project-path ".claude/skills" --global-path "~/.claude/skills"
./target/debug/skills-mgr agent add cursor --project-path ".cursor/skills" --global-path "~/.cursor/skills"
./target/debug/skills-mgr agent list

# Create skills
./target/debug/skills-mgr skill create rust-engineer --description "Expert Rust development"
./target/debug/skills-mgr skill create react-specialist --description "React development patterns"
./target/debug/skills-mgr skill create code-review --description "Code review best practices"
./target/debug/skills-mgr skill list

# Create profiles
./target/debug/skills-mgr profile create rust --add rust-engineer
./target/debug/skills-mgr profile create react --add react-specialist
./target/debug/skills-mgr profile edit base --add code-review
./target/debug/skills-mgr profile list

# Activate
./target/debug/skills-mgr profile activate rust
./target/debug/skills-mgr status

# Verify files exist
ls -la .claude/skills/
ls -la .cursor/skills/

# Deactivate
./target/debug/skills-mgr profile deactivate rust
./target/debug/skills-mgr status

# Check log
./target/debug/skills-mgr log
```

Expected: Full workflow succeeds — skills placed in agent directories, removed on deactivation, logs recorded.

- [ ] **Step 3: Final commit with any fixes**

```bash
git add -A
git commit -m "fix: address issues found during end-to-end smoke test"
```

---

## Chunk 8: CI/CD Pipeline

### Task 16: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create CI workflow**

Create `.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [master]
  pull_request:
    branches: [master]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check & Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo registry & build
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy lints
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Run tests
        run: cargo test --workspace

  frontend:
    name: Frontend Lint & Build
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: crates/skills-gui
    steps:
      - uses: actions/checkout@v4

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: crates/skills-gui/pnpm-lock.yaml

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: TypeScript check
        run: pnpm tsc --noEmit

      - name: Lint
        run: pnpm lint

      - name: Build frontend
        run: pnpm build
```

- [ ] **Step 2: Create release workflow**

Create `.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            label: macos-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            label: macos-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            label: linux-x64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            label: windows-x64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install system dependencies (Linux)
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22

      - name: Install Tauri CLI
        run: cargo install tauri-cli --version "^2"

      - name: Install frontend dependencies
        working-directory: crates/skills-gui
        run: pnpm install --frozen-lockfile

      - name: Build Tauri app
        working-directory: crates/skills-gui
        run: cargo tauri build --target ${{ matrix.target }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: skills-mgr-${{ matrix.label }}
          path: |
            target/${{ matrix.target }}/release/bundle/**/*
            target/${{ matrix.target }}/release/skills-mgr*

  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: artifacts/**/*
```

- [ ] **Step 3: Verify CI workflow syntax**

Run: `gh workflow list` (if gh CLI is available)
Or validate YAML syntax manually.

- [ ] **Step 4: Commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions CI and release workflows"
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1: Foundation | Tasks 1-4 | Workspace, config parsing, SQLite, skill registry |
| 2: Engines | Tasks 5-7 | Profile resolution, placement engine, operation logging |
| 3: CLI | Task 8 | Full CLI with all commands |
| 4: MCP | Task 9 | MCP server scaffold |
| 5: UI Design | Task 10 ✅ | Pencil mockups: 16 screens × dark/light = 32 frames in `designs/skills-mgr-ui.pen` |
| 6: GUI Foundation | Tasks 11-12 | Tauri 2 + React + shadcn/ui + IPC commands |
| 7: GUI Views + Polish | Tasks 13-15 | App shell, pages (implement from Pencil designs), teaching skill, smoke test |
| 8: CI/CD | Task 16 | GitHub Actions CI + cross-platform release workflow |

**After this plan completes:** The MVP is functional with CLI, GUI shell, and core profile/activation workflow. The MCP server scaffold exists but needs full tool implementations (follow-up task). The GUI page stubs need to be fleshed out with full CRUD UI (follow-up task). CI runs on every push/PR, and tagged releases build cross-platform binaries automatically.
