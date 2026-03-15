# Skill Delegation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Discover unmanaged skills in agent paths, delegate them to skills-mgr for centralized management, and enable linking local skills to remote URLs for upstream sync.

**Architecture:** A new `discovery` module scans configured agent global/project paths for SKILL.md files not managed by skills-mgr. Discovered skills can be "delegated" — imported into the registry as Local type and assigned to a profile. Any local skill can be linked to a remote GitHub URL via a new `link_remote()` registry method, enabling `skill update` to pull from upstream. Auto-scan is a configurable setting that runs on app startup.

**Tech Stack:** Rust (skills-core, skills-cli, skills-mcp, skills-gui/src-tauri), TypeScript/React (skills-gui frontend), SQLite (via SeaORM), TOML configs, Tauri v2, Zod

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `crates/skills-core/src/discovery.rs` | Core discovery engine: scan agent paths, diff against registry/placements, return unmanaged skills |
| `crates/skills-cli/src/commands/discover.rs` | CLI `skill discover` and `skill link-remote` handlers |

### Modified Files
| File | Changes |
|------|---------|
| `crates/skills-core/src/config.rs` | Add `original_agent_path` to `SkillSource`, add `ScanSettings` to `AppSettings` |
| `crates/skills-core/src/registry.rs` | Add `link_remote()` and `delegate()` methods |
| `crates/skills-core/src/lib.rs` | Export `discovery` module |
| `crates/skills-cli/src/main.rs` | Add `Discover`, `LinkRemote` subcommands under `Skill` |
| `crates/skills-cli/src/commands/mod.rs` | Add `pub mod discover;` |
| `crates/skills-gui/src-tauri/src/commands.rs` | Add `scan_skills`, `delegate_skills`, `link_remote` Tauri commands |
| `crates/skills-gui/src-tauri/src/lib.rs` | Register new commands |
| `crates/skills-gui/src/lib/schemas.ts` | Add `DiscoveredSkillSchema` |
| `crates/skills-gui/src/lib/api.ts` | Add `scanSkills()`, `delegateSkills()`, `linkRemote()` |
| `crates/skills-gui/src/pages/Skills.tsx` | Add Discover tab with scan results, delegation flow; add "Link to Remote" in detail panel |
| `crates/skills-gui/src/pages/Settings.tsx` | Add "Skill Discovery" settings section |
| `crates/skills-mcp/src/lib.rs` | Add `discover_skills`, `delegate_skill`, `link_remote` MCP tools |

---

## Chunk 1: Core Data Model + Discovery Engine

### Task 1: Extend SkillSource with original_agent_path

**Files:**
- Modify: `crates/skills-core/src/config.rs:82-92` (SkillSource struct)
- Test: `crates/skills-core/src/config.rs:326-350` (existing test_sources_config_roundtrip)

- [ ] **Step 1: Write test for new field roundtrip**

Add to existing `test_sources_config_roundtrip` in `config.rs`:

```rust
#[test]
fn test_sources_config_original_agent_path_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sources.toml");

    let config = SourcesConfig {
        skills: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(
                "delegated-skill".into(),
                SkillSource {
                    source_type: SourceType::Local,
                    url: None,
                    path: None,
                    git_ref: None,
                    hash: Some("sha256:abc123".into()),
                    updated_at: Some("2026-03-14T12:00:00Z".into()),
                    original_agent_path: Some("~/.claude/skills/delegated-skill".into()),
                },
            );
            m
        },
    };
    config.save(&path).unwrap();
    let loaded = SourcesConfig::load(&path).unwrap();
    assert_eq!(
        loaded.skills["delegated-skill"].original_agent_path,
        Some("~/.claude/skills/delegated-skill".into())
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p skills-core test_sources_config_original_agent_path_roundtrip`
Expected: FAIL — `original_agent_path` field doesn't exist on `SkillSource`

- [ ] **Step 3: Add field to SkillSource**

In `crates/skills-core/src/config.rs`, add to `SkillSource` struct (after line 91):

```rust
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
    #[serde(default)]
    pub original_agent_path: Option<String>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p skills-core test_sources_config_original_agent_path`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/skills-core/src/config.rs
git commit -m "feat: add original_agent_path to SkillSource for delegation tracking"
```

---

### Task 2: Add ScanSettings to AppSettings

**Files:**
- Modify: `crates/skills-core/src/config.rs:185-244` (AppSettings)

- [ ] **Step 1: Write test for scan settings roundtrip**

```rust
#[test]
fn test_app_settings_scan_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("settings.toml");

    let settings = AppSettings {
        mcp: McpSettings::default(),
        git_sync: GitSyncSettings::default(),
        scan: ScanSettings {
            auto_scan_on_startup: true,
        },
    };
    settings.save(&path).unwrap();
    let loaded = AppSettings::load(&path).unwrap();
    assert!(loaded.scan.auto_scan_on_startup);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p skills-core test_app_settings_scan_roundtrip`
Expected: FAIL — `ScanSettings` doesn't exist

- [ ] **Step 3: Add ScanSettings struct and field**

In `crates/skills-core/src/config.rs`, after `GitSyncSettings` (after line 227):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanSettings {
    #[serde(default)]
    pub auto_scan_on_startup: bool,
}
```

Add to `AppSettings`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub mcp: McpSettings,
    #[serde(default)]
    pub git_sync: GitSyncSettings,
    #[serde(default)]
    pub scan: ScanSettings,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p skills-core test_app_settings_scan_roundtrip`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/skills-core/src/config.rs
git commit -m "feat: add ScanSettings to AppSettings for auto-scan configuration"
```

---

### Task 3: Create discovery module — DiscoveredSkill type and scan_agent_path

**Files:**
- Create: `crates/skills-core/src/discovery.rs`
- Modify: `crates/skills-core/src/lib.rs`

- [ ] **Step 1: Write test for scanning a single agent global path**

Create `crates/skills-core/src/discovery.rs`:

```rust
use std::path::{Path, PathBuf};
use crate::config::{AgentsConfig, AppDirs, SourcesConfig};
use crate::registry::Registry;

/// A skill discovered in an agent path that is not managed by skills-mgr.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name (directory name).
    pub name: String,
    /// Description from SKILL.md frontmatter, if parseable.
    pub description: Option<String>,
    /// Agent that owns this path.
    pub agent_name: String,
    /// Full absolute path where the skill was found.
    pub found_path: PathBuf,
    /// Whether this is a global or project-scoped discovery.
    pub scope: DiscoveryScope,
    /// File list within the skill directory.
    pub files: Vec<String>,
    /// Total bytes of skill content.
    pub total_bytes: u64,
    /// Estimated token count (~4 bytes/token).
    pub token_estimate: u64,
    /// Whether this skill already exists in the registry (potential conflict).
    pub exists_in_registry: bool,
}

/// Scope of discovery — global (agent's global_path) or project-specific.
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryScope {
    Global,
    Project(String), // project path
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, AppDirs, Registry) {
        let tmp = TempDir::new().unwrap();
        let dirs = AppDirs::new(tmp.path().to_path_buf());
        dirs.ensure_dirs().unwrap();
        let reg = Registry::new(dirs.clone());
        (tmp, dirs, reg)
    }

    #[test]
    fn test_scan_global_finds_unmanaged_skills() {
        let (tmp, dirs, reg) = setup_test_env();

        // Create a fake agent global path with a skill
        let global_dir = tmp.path().join("agent-global");
        let skill_dir = global_dir.join("external-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: external-skill\ndescription: Found externally\n---\nContent here",
        ).unwrap();

        let results = scan_agent_path(
            &dirs,
            &reg,
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        ).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "external-skill");
        assert_eq!(results[0].description, Some("Found externally".into()));
        assert_eq!(results[0].agent_name, "test-agent");
        assert_eq!(results[0].scope, DiscoveryScope::Global);
        assert!(!results[0].exists_in_registry);
    }

    #[test]
    fn test_scan_global_skips_managed_skills() {
        let (tmp, dirs, reg) = setup_test_env();

        // Create a managed skill in registry
        reg.create("managed-skill", "Managed by us").unwrap();

        // Place same-named skill in agent global path
        let global_dir = tmp.path().join("agent-global");
        let skill_dir = global_dir.join("managed-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: managed-skill\ndescription: Managed\n---\n",
        ).unwrap();

        let results = scan_agent_path(
            &dirs,
            &reg,
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        ).unwrap();

        // Skill exists in registry — still discovered but flagged
        assert_eq!(results.len(), 1);
        assert!(results[0].exists_in_registry);
    }

    #[test]
    fn test_scan_skips_dirs_without_skill_md() {
        let (tmp, dirs, reg) = setup_test_env();

        let global_dir = tmp.path().join("agent-global");
        let random_dir = global_dir.join("not-a-skill");
        std::fs::create_dir_all(&random_dir).unwrap();
        std::fs::write(random_dir.join("README.md"), "not a skill").unwrap();

        let results = scan_agent_path(
            &dirs,
            &reg,
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        ).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_nonexistent_path_returns_empty() {
        let (_tmp, dirs, reg) = setup_test_env();

        let results = scan_agent_path(
            &dirs,
            &reg,
            "test-agent",
            Path::new("/nonexistent/path"),
            DiscoveryScope::Global,
        ).unwrap();

        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p skills-core test_scan_global_finds_unmanaged_skills`
Expected: FAIL — `scan_agent_path` doesn't exist

- [ ] **Step 3: Implement scan_agent_path**

Add to `crates/skills-core/src/discovery.rs` (before the tests module):

```rust
use anyhow::Result;

/// Scan a single agent path (global or project) for skills not managed by skills-mgr.
///
/// Returns discovered skills. Skills already in the registry are flagged with
/// `exists_in_registry = true` but still returned (for conflict display).
pub fn scan_agent_path(
    dirs: &AppDirs,
    registry: &Registry,
    agent_name: &str,
    scan_path: &Path,
    scope: DiscoveryScope,
) -> Result<Vec<DiscoveredSkill>> {
    if !scan_path.exists() {
        return Ok(vec![]);
    }

    let sources = SourcesConfig::load(&dirs.sources_toml()).unwrap_or_default();
    let mut discovered = Vec::new();

    let entries = match std::fs::read_dir(scan_path) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let skill_dir = entry.path();
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip if this path is already tracked as a delegated skill's original_agent_path
        let is_tracked_delegation = sources.skills.values().any(|s| {
            s.original_agent_path.as_deref()
                == Some(&skill_dir.to_string_lossy())
        });
        if is_tracked_delegation {
            continue;
        }

        let description = parse_skill_description(&skill_md);
        let (files, total_bytes, token_estimate) = list_skill_files(&skill_dir);
        let exists_in_registry = registry.exists(&name);

        discovered.push(DiscoveredSkill {
            name,
            description,
            agent_name: agent_name.to_string(),
            found_path: skill_dir,
            scope: scope.clone(),
            files,
            total_bytes,
            token_estimate,
            exists_in_registry,
        });
    }

    discovered.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(discovered)
}

/// Parse description from a SKILL.md frontmatter.
fn parse_skill_description(skill_md: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_md).ok()?;
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }
    let end = content[3..].find("---")?;
    let frontmatter = &content[3..3 + end];
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(desc) = line.strip_prefix("description:") {
            return Some(desc.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

/// List files and compute size stats for a skill directory.
/// Lightweight version that doesn't error — returns empty on failure.
fn list_skill_files(dir: &Path) -> (Vec<String>, u64, u64) {
    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    collect_files(dir, dir, &mut files, &mut total_bytes);
    files.sort();
    (files, total_bytes, total_bytes / 4)
}

fn collect_files(base: &Path, current: &Path, files: &mut Vec<String>, total_bytes: &mut u64) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, files, total_bytes);
        } else {
            if let Ok(meta) = entry.metadata() {
                *total_bytes += meta.len();
            }
            if let Ok(rel) = path.strip_prefix(base) {
                files.push(rel.to_string_lossy().to_string());
            }
        }
    }
}
```

- [ ] **Step 4: Export discovery module**

In `crates/skills-core/src/lib.rs`, add:

```rust
pub mod discovery;
```

- [ ] **Step 5: Run all discovery tests**

Run: `cargo test -p skills-core discovery`
Expected: All 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skills-core/src/discovery.rs crates/skills-core/src/lib.rs
git commit -m "feat: add discovery module with scan_agent_path"
```

---

### Task 4: Add scan_all_agents high-level function

**Files:**
- Modify: `crates/skills-core/src/discovery.rs`

- [ ] **Step 1: Write test for scan_all_agents**

```rust
#[test]
fn test_scan_all_agents_global() {
    let (tmp, dirs, reg) = setup_test_env();

    // Create agent config
    let global_dir = tmp.path().join("claude-global");
    let skill_dir = global_dir.join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: Test\n---\n",
    ).unwrap();

    let agents_config = AgentsConfig {
        agents: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(
                "claude".into(),
                crate::config::AgentDef {
                    project_path: ".claude/skills".into(),
                    global_path: global_dir.to_string_lossy().to_string(),
                    enabled: true,
                },
            );
            m
        },
    };

    let results = scan_all_agents(&dirs, &reg, &agents_config, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "my-skill");
    assert_eq!(results[0].agent_name, "claude");
    assert_eq!(results[0].scope, DiscoveryScope::Global);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p skills-core test_scan_all_agents_global`
Expected: FAIL — `scan_all_agents` doesn't exist

- [ ] **Step 3: Implement scan_all_agents**

Add to `crates/skills-core/src/discovery.rs`:

```rust
use crate::placements;

/// Scan all configured agent paths for unmanaged skills.
///
/// Scans each enabled agent's global_path and, for each provided project path,
/// the agent's project_path resolved relative to that project.
///
/// `project_paths` is a list of known project directories to scan for
/// project-level skills. Pass empty slice to scan only global paths.
pub fn scan_all_agents(
    dirs: &AppDirs,
    registry: &Registry,
    agents_config: &AgentsConfig,
    project_paths: &[String],
) -> Result<Vec<DiscoveredSkill>> {
    let mut all = Vec::new();

    for (agent_name, agent_def) in &agents_config.agents {
        if !agent_def.enabled {
            continue;
        }

        // Scan global path
        let global_expanded = placements::expand_tilde(&agent_def.global_path);
        let global_path = PathBuf::from(&global_expanded);
        let global_results = scan_agent_path(
            dirs,
            registry,
            agent_name,
            &global_path,
            DiscoveryScope::Global,
        )?;
        all.extend(global_results);

        // Scan project paths
        for project_path in project_paths {
            let project_dir = PathBuf::from(project_path);
            let agent_project_dir = project_dir.join(&agent_def.project_path);
            let project_results = scan_agent_path(
                dirs,
                registry,
                agent_name,
                &agent_project_dir,
                DiscoveryScope::Project(project_path.clone()),
            )?;
            all.extend(project_results);
        }
    }

    Ok(all)
}
```

Note: This requires `expand_tilde` to be `pub` in `placements.rs`. If it's currently `fn`, change to `pub fn`.

- [ ] **Step 4: Make expand_tilde public in placements.rs if needed**

In `crates/skills-core/src/placements.rs`, ensure:

```rust
pub fn expand_tilde(path: &str) -> String {
```

- [ ] **Step 5: Run test**

Run: `cargo test -p skills-core test_scan_all_agents_global`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skills-core/src/discovery.rs crates/skills-core/src/placements.rs
git commit -m "feat: add scan_all_agents for full agent path discovery"
```

---

## Chunk 2: Registry Delegation + Remote Linking

### Task 5: Add delegate() method to Registry

**Files:**
- Modify: `crates/skills-core/src/registry.rs`

This method imports a discovered skill into the registry and records its original agent path.

- [ ] **Step 1: Write test**

Add to `crates/skills-core/src/registry.rs` tests:

```rust
#[test]
fn test_delegate_skill() {
    let tmp_src = TempDir::new().unwrap();
    let skill_dir = tmp_src.path().join("ext-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: ext-skill\ndescription: External\n---\nContent",
    ).unwrap();

    let (_tmp, reg) = setup_test_registry();
    let name = reg
        .delegate(&skill_dir, "claude", "~/.claude/skills/ext-skill")
        .unwrap();
    assert_eq!(name, "ext-skill");
    assert!(reg.exists("ext-skill"));

    // Check source metadata
    let sources = SourcesConfig::load(&reg.dirs.sources_toml()).unwrap();
    let src = &sources.skills["ext-skill"];
    assert_eq!(src.source_type, SourceType::Local);
    assert_eq!(
        src.original_agent_path,
        Some("~/.claude/skills/ext-skill".into())
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p skills-core test_delegate_skill`
Expected: FAIL — `delegate` method doesn't exist

- [ ] **Step 3: Implement delegate()**

Add to `impl Registry` in `crates/skills-core/src/registry.rs`:

```rust
/// Import a discovered skill into the registry (delegation).
///
/// Copies the skill directory into the registry and records the original
/// agent path in sources.toml for tracking.
pub fn delegate(
    &self,
    source_dir: &Path,
    agent_name: &str,
    original_path: &str,
) -> Result<String> {
    let skill_md = source_dir.join("SKILL.md");
    if !skill_md.exists() {
        bail!("No SKILL.md found at {}", source_dir.display());
    }

    let name = source_dir
        .file_name()
        .context("Invalid source path")?
        .to_string_lossy()
        .to_string();

    let dest = self.dirs.registry().join(&name);
    if dest.exists() {
        bail!("Skill '{}' already exists in registry", name);
    }

    copy_dir_recursive(source_dir, &dest)?;

    let hash = compute_tree_hash(&dest)?;
    let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
    sources.skills.insert(
        name.clone(),
        SkillSource {
            source_type: SourceType::Local,
            url: None,
            path: None,
            git_ref: None,
            hash: Some(hash),
            updated_at: Some(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string(),
            ),
            original_agent_path: Some(original_path.to_string()),
        },
    );
    sources.save(&self.dirs.sources_toml())?;

    Ok(name)
}
```

Note: Requires adding `use crate::config::SkillSource;` import if not already present. The struct already has `original_agent_path` from Task 1.

- [ ] **Step 4: Run test**

Run: `cargo test -p skills-core test_delegate_skill`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/skills-core/src/registry.rs
git commit -m "feat: add delegate() method to Registry for skill delegation"
```

---

### Task 6: Add link_remote() method to Registry

**Files:**
- Modify: `crates/skills-core/src/registry.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn test_link_remote_to_local_skill() {
    let (_tmp, reg) = setup_test_registry();
    reg.create("my-local-skill", "A local skill").unwrap();

    reg.link_remote(
        "my-local-skill",
        "https://github.com/owner/repo",
        Some("skills/my-local-skill"),
        "main",
    ).unwrap();

    let sources = SourcesConfig::load(&reg.dirs.sources_toml()).unwrap();
    let src = &sources.skills["my-local-skill"];
    assert_eq!(src.source_type, SourceType::Git);
    assert_eq!(src.url, Some("https://github.com/owner/repo".into()));
    assert_eq!(src.git_ref, Some("main".into()));
    assert_eq!(src.path, Some("skills/my-local-skill".into()));
}

#[test]
fn test_unlink_remote() {
    let (_tmp, reg) = setup_test_registry();
    reg.create("linked-skill", "Linked").unwrap();
    reg.link_remote("linked-skill", "https://github.com/o/r", None, "main").unwrap();

    reg.unlink_remote("linked-skill").unwrap();

    let sources = SourcesConfig::load(&reg.dirs.sources_toml()).unwrap();
    let src = &sources.skills["linked-skill"];
    assert_eq!(src.source_type, SourceType::Local);
    assert!(src.url.is_none());
    assert!(src.git_ref.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p skills-core test_link_remote test_unlink_remote`
Expected: FAIL — methods don't exist

- [ ] **Step 3: Implement link_remote() and unlink_remote()**

Add to `impl Registry`:

```rust
/// Link a local skill to a remote GitHub URL for upstream sync.
///
/// Changes the source_type from Local to Git and records the URL/ref.
/// After linking, `skill update <name>` can pull from the remote.
pub fn link_remote(
    &self,
    name: &str,
    url: &str,
    subpath: Option<&str>,
    git_ref: &str,
) -> Result<()> {
    if !self.exists(name) {
        bail!("Skill '{}' not found in registry", name);
    }

    let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
    let entry = sources.skills.entry(name.to_string()).or_insert_with(|| {
        SkillSource {
            source_type: SourceType::Local,
            url: None,
            path: None,
            git_ref: None,
            hash: None,
            updated_at: None,
            original_agent_path: None,
        }
    });

    entry.source_type = SourceType::Git;
    entry.url = Some(url.to_string());
    entry.path = subpath.map(|s| s.to_string());
    entry.git_ref = Some(git_ref.to_string());
    entry.updated_at = Some(
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
    );

    sources.save(&self.dirs.sources_toml())?;
    Ok(())
}

/// Unlink a skill from its remote URL, reverting to Local type.
pub fn unlink_remote(&self, name: &str) -> Result<()> {
    if !self.exists(name) {
        bail!("Skill '{}' not found in registry", name);
    }

    let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
    if let Some(entry) = sources.skills.get_mut(name) {
        entry.source_type = SourceType::Local;
        entry.url = None;
        entry.path = None;
        entry.git_ref = None;
    }

    sources.save(&self.dirs.sources_toml())?;
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p skills-core test_link_remote test_unlink_remote`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p skills-core`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/skills-core/src/registry.rs
git commit -m "feat: add link_remote/unlink_remote for local-to-remote skill linking"
```

---

## Chunk 3: CLI Commands

### Task 7: Add CLI discover and link-remote subcommands

**Files:**
- Modify: `crates/skills-cli/src/main.rs:71-98` (SkillAction enum)
- Create: `crates/skills-cli/src/commands/discover.rs`
- Modify: `crates/skills-cli/src/commands/mod.rs`

- [ ] **Step 1: Add subcommand variants to SkillAction**

In `crates/skills-cli/src/main.rs`, add to `SkillAction` enum:

```rust
#[derive(Subcommand)]
pub enum SkillAction {
    // ... existing variants ...
    /// Discover unmanaged skills in agent paths
    Discover {
        /// Only scan global paths (skip project paths)
        #[arg(long)]
        global_only: bool,
    },
    /// Link a local skill to a remote GitHub URL for sync
    LinkRemote {
        name: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        subpath: Option<String>,
        #[arg(long, default_value = "main")]
        git_ref: String,
    },
    /// Unlink a skill from its remote URL
    UnlinkRemote {
        name: String,
    },
}
```

- [ ] **Step 2: Create discover command handler**

Create `crates/skills-cli/src/commands/discover.rs`:

```rust
use anyhow::Result;
use skills_core::config::{AgentsConfig, AppDirs};
use skills_core::discovery::{self, DiscoveryScope};
use skills_core::{Database, Registry};

pub async fn run_discover(dirs: &AppDirs, db: &Database, global_only: bool) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;

    if agents_config.agents.is_empty() {
        println!("No agents configured. Add agents first with `skills-mgr agent add`.");
        return Ok(());
    }

    let project_paths = if global_only {
        vec![]
    } else {
        db.list_all_projects()
            .await?
            .into_iter()
            .filter(|p| p.path != "__global__")
            .map(|p| p.path)
            .collect()
    };

    let discovered = discovery::scan_all_agents(dirs, &registry, &agents_config, &project_paths)?;

    if discovered.is_empty() {
        println!("No unmanaged skills found in agent paths.");
        return Ok(());
    }

    println!("Found {} unmanaged skill(s):\n", discovered.len());

    let mut current_scope = None;
    for skill in &discovered {
        let scope_label = match &skill.scope {
            DiscoveryScope::Global => "Global".to_string(),
            DiscoveryScope::Project(p) => format!("Project: {}", p),
        };
        if current_scope.as_ref() != Some(&scope_label) {
            if current_scope.is_some() {
                println!();
            }
            println!("  [{}]", scope_label);
            current_scope = Some(scope_label);
        }

        let conflict = if skill.exists_in_registry { " (exists in registry)" } else { "" };
        println!(
            "    {} — {} ({} files, ~{} tokens) via {}{}",
            skill.name,
            skill.description.as_deref().unwrap_or("No description"),
            skill.files.len(),
            skill.token_estimate,
            skill.agent_name,
            conflict,
        );
    }

    println!("\nUse `skills-mgr skill add <path>` to import individual skills.");
    Ok(())
}

pub fn run_link_remote(
    dirs: &AppDirs,
    name: &str,
    url: &str,
    subpath: Option<&str>,
    git_ref: &str,
) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    registry.link_remote(name, url, subpath, git_ref)?;
    println!("Linked '{}' to remote: {} (ref: {})", name, url, git_ref);
    Ok(())
}

pub fn run_unlink_remote(dirs: &AppDirs, name: &str) -> Result<()> {
    let registry = Registry::new(dirs.clone());
    registry.unlink_remote(name)?;
    println!("Unlinked '{}' from remote — reverted to local", name);
    Ok(())
}
```

- [ ] **Step 3: Register module and wire up commands**

In `crates/skills-cli/src/commands/mod.rs`, add:

```rust
pub mod discover;
```

In `crates/skills-cli/src/main.rs`, add match arms in the `Commands::Skill` handler (inside `commands::skill::run` or in main match):

```rust
SkillAction::Discover { global_only } => {
    commands::discover::run_discover(&dirs, &db, global_only).await?;
}
SkillAction::LinkRemote { name, url, subpath, git_ref } => {
    commands::discover::run_link_remote(&dirs, &name, &url, subpath.as_deref(), &git_ref)?;
}
SkillAction::UnlinkRemote { name } => {
    commands::discover::run_unlink_remote(&dirs, &name)?;
}
```

- [ ] **Step 4: Verify build**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/skills-cli/src/main.rs crates/skills-cli/src/commands/discover.rs crates/skills-cli/src/commands/mod.rs
git commit -m "feat: add CLI discover, link-remote, and unlink-remote subcommands"
```

---

## Chunk 4: Tauri Commands + Frontend Schemas/API

### Task 8: Add Tauri scan/delegate/link-remote commands

**Files:**
- Modify: `crates/skills-gui/src-tauri/src/commands.rs`
- Modify: `crates/skills-gui/src-tauri/src/lib.rs`

- [ ] **Step 1: Add DiscoveredSkillInfo serialization struct**

In `crates/skills-gui/src-tauri/src/commands.rs`, add:

```rust
#[derive(Serialize)]
pub struct DiscoveredSkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub agent_name: String,
    pub found_path: String,
    pub scope: String,         // "global" or project path
    pub files: Vec<String>,
    pub total_bytes: u64,
    pub token_estimate: u64,
    pub exists_in_registry: bool,
}
```

- [ ] **Step 2: Implement scan_skills command**

```rust
#[tauri::command]
pub async fn scan_skills(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredSkillInfo>, String> {
    let dirs = &state.dirs;
    let registry = Registry::new(dirs.clone());
    let agents_config = AgentsConfig::load(&dirs.agents_toml()).map_err(|e| e.to_string())?;

    let project_paths: Vec<String> = state
        .db
        .list_all_projects()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p.path != "__global__")
        .map(|p| p.path)
        .collect();

    let discovered = skills_core::discovery::scan_all_agents(
        dirs,
        &registry,
        &agents_config,
        &project_paths,
    )
    .map_err(|e| e.to_string())?;

    Ok(discovered
        .into_iter()
        .map(|d| {
            let scope = match &d.scope {
                skills_core::discovery::DiscoveryScope::Global => "global".to_string(),
                skills_core::discovery::DiscoveryScope::Project(p) => p.clone(),
            };
            DiscoveredSkillInfo {
                name: d.name,
                description: d.description,
                agent_name: d.agent_name,
                found_path: d.found_path.to_string_lossy().to_string(),
                scope,
                files: d.files,
                total_bytes: d.total_bytes,
                token_estimate: d.token_estimate,
                exists_in_registry: d.exists_in_registry,
            }
        })
        .collect())
}
```

- [ ] **Step 3: Implement delegate_skills command**

```rust
#[tauri::command]
pub async fn delegate_skills(
    state: State<'_, AppState>,
    skills: Vec<DelegateRequest>,
    profile_name: String,
    create_profile: bool,
    profile_description: Option<String>,
) -> Result<String, String> {
    let dirs = &state.dirs;
    let registry = Registry::new(dirs.clone());

    // Create profile if requested
    if create_profile {
        let mut profiles_config =
            ProfilesConfig::load(&dirs.profiles_toml()).map_err(|e| e.to_string())?;
        if profiles_config.profiles.contains_key(&profile_name) {
            return Err(format!("Profile '{}' already exists", profile_name));
        }
        profiles_config.profiles.insert(
            profile_name.clone(),
            skills_core::config::ProfileDef {
                description: profile_description,
                skills: vec![],
                includes: vec![],
            },
        );
        profiles_config
            .save(&dirs.profiles_toml())
            .map_err(|e| e.to_string())?;
    }

    let mut delegated = Vec::new();
    for req in &skills {
        let source_path = std::path::PathBuf::from(&req.found_path);
        match registry.delegate(&source_path, &req.agent_name, &req.found_path) {
            Ok(name) => delegated.push(name),
            Err(e) => {
                // Skip skills that already exist if user chose to skip conflicts
                if !e.to_string().contains("already exists") {
                    return Err(e.to_string());
                }
            }
        }
    }

    // Add delegated skills to the profile
    if !delegated.is_empty() {
        let mut profiles_config =
            ProfilesConfig::load(&dirs.profiles_toml()).map_err(|e| e.to_string())?;
        if let Some(profile) = profiles_config.profiles.get_mut(&profile_name) {
            for name in &delegated {
                if !profile.skills.contains(name) {
                    profile.skills.push(name.clone());
                }
            }
        }
        profiles_config
            .save(&dirs.profiles_toml())
            .map_err(|e| e.to_string())?;
    }

    logging::log(
        &state.db,
        logging::LogEntry {
            source: logging::Source::Gui,
            agent_name: None,
            operation: "skill_delegate",
            params: None,
            project_path: None,
            result: "success",
            details: &format!(
                "Delegated {} skill(s) to profile '{}'",
                delegated.len(),
                profile_name
            ),
        },
    )
    .await;

    Ok(format!(
        "Delegated {} skill(s) to profile '{}'",
        delegated.len(),
        profile_name
    ))
}

#[derive(Deserialize)]
pub struct DelegateRequest {
    pub name: String,
    pub agent_name: String,
    pub found_path: String,
}
```

- [ ] **Step 4: Implement link_remote and unlink_remote commands**

```rust
#[tauri::command]
pub async fn link_remote(
    state: State<'_, AppState>,
    name: String,
    url: String,
    subpath: Option<String>,
    git_ref: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry
        .link_remote(&name, &url, subpath.as_deref(), &git_ref)
        .map_err(|e| e.to_string())?;

    logging::log(
        &state.db,
        logging::LogEntry {
            source: logging::Source::Gui,
            agent_name: None,
            operation: "skill_link_remote",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Linked '{}' to {}", name, url),
        },
    )
    .await;

    Ok(format!("Linked '{}' to remote: {}", name, url))
}

#[tauri::command]
pub async fn unlink_remote(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let registry = Registry::new(state.dirs.clone());
    registry.unlink_remote(&name).map_err(|e| e.to_string())?;

    logging::log(
        &state.db,
        logging::LogEntry {
            source: logging::Source::Gui,
            agent_name: None,
            operation: "skill_unlink_remote",
            params: None,
            project_path: None,
            result: "success",
            details: &format!("Unlinked '{}' from remote", name),
        },
    )
    .await;

    Ok(format!("Unlinked '{}' from remote", name))
}
```

- [ ] **Step 5: Register commands in lib.rs**

In `crates/skills-gui/src-tauri/src/lib.rs`, add to `invoke_handler`:

```rust
scan_skills,
delegate_skills,
link_remote,
unlink_remote,
```

- [ ] **Step 6: Verify build**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 7: Commit**

```bash
git add crates/skills-gui/src-tauri/src/commands.rs crates/skills-gui/src-tauri/src/lib.rs
git commit -m "feat: add Tauri commands for scan, delegate, link/unlink remote"
```

---

### Task 9: Add frontend schemas and API functions

**Files:**
- Modify: `crates/skills-gui/src/lib/schemas.ts`
- Modify: `crates/skills-gui/src/lib/api.ts`

- [ ] **Step 1: Add DiscoveredSkillSchema to schemas.ts**

```typescript
export const DiscoveredSkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  agent_name: z.string(),
  found_path: z.string(),
  scope: z.string(),
  files: z.array(z.string()),
  total_bytes: z.number(),
  token_estimate: z.number(),
  exists_in_registry: z.boolean(),
})

export type DiscoveredSkill = z.infer<typeof DiscoveredSkillSchema>
```

- [ ] **Step 2: Add API functions to api.ts**

```typescript
// --- Discovery & Delegation ---

export async function scanSkills() {
  const data = await invoke("scan_skills")
  return z.array(DiscoveredSkillSchema).parse(data)
}

export interface DelegateRequest {
  name: string
  agent_name: string
  found_path: string
}

export async function delegateSkills(
  skills: DelegateRequest[],
  profileName: string,
  createProfile: boolean,
  profileDescription?: string,
) {
  return await invoke("delegate_skills", {
    skills,
    profileName,
    createProfile,
    profileDescription,
  }) as string
}

export async function linkRemote(
  name: string,
  url: string,
  gitRef: string,
  subpath?: string,
) {
  return await invoke("link_remote", { name, url, subpath, gitRef }) as string
}

export async function unlinkRemote(name: string) {
  return await invoke("unlink_remote", { name }) as string
}
```

Add `DiscoveredSkillSchema` to the import in `api.ts`.

- [ ] **Step 3: TypeScript check**

Run: `cd crates/skills-gui && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/skills-gui/src/lib/schemas.ts crates/skills-gui/src/lib/api.ts
git commit -m "feat: add frontend schemas and API functions for discovery/delegation"
```

---

## Chunk 5: GUI — Discovery Tab in Skills Page

### Task 10: Add Discover tab to Skills page

**Files:**
- Modify: `crates/skills-gui/src/pages/Skills.tsx`

This adds a tab switcher at the top of the Skills page: "Registry" (existing content) and "Discover" (new scan results). The Discover tab has a "Scan Now" button, shows grouped results, and has a "Delegate" action that opens a profile picker dialog.

- [ ] **Step 1: Add tab state and scan query**

At the top of the `Skills()` component, add:

```typescript
const [tab, setTab] = useState<"registry" | "discover">("registry")

// Discovery state
const {
  data: discovered,
  isLoading: isScanning,
  refetch: runScan,
} = useQuery({
  queryKey: ["discoveredSkills"],
  queryFn: scanSkills,
  enabled: false, // manual trigger only
})
```

Import `scanSkills`, `delegateSkills`, `DelegateRequest` from `@/lib/api` and `DiscoveredSkill` from `@/lib/schemas`.

- [ ] **Step 2: Add tab switcher UI**

Below the header search bar section but before the skill cards grid, add tab buttons:

```tsx
{/* Tab switcher */}
<div className="shrink-0 flex gap-1 rounded-lg bg-muted p-1 mb-4">
  <button
    onClick={() => setTab("registry")}
    className={`flex-1 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
      tab === "registry"
        ? "bg-background text-foreground shadow-sm"
        : "text-muted-foreground hover:text-foreground"
    }`}
  >
    Registry ({skills?.length ?? 0})
  </button>
  <button
    onClick={() => {
      setTab("discover")
      if (!discovered) runScan()
    }}
    className={`flex-1 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
      tab === "discover"
        ? "bg-background text-foreground shadow-sm"
        : "text-muted-foreground hover:text-foreground"
    }`}
  >
    Discover
    {discovered && discovered.length > 0 && (
      <span className="ml-1.5 rounded-full bg-primary/15 px-1.5 text-[10px] font-bold text-primary">
        {discovered.length}
      </span>
    )}
  </button>
</div>
```

- [ ] **Step 3: Add discovery results panel**

Wrap existing skill cards grid in `{tab === "registry" && (...)}`. Add discover tab content:

```tsx
{tab === "discover" && (
  <div className="flex-1 min-h-0 overflow-y-auto space-y-4">
    {/* Scan button */}
    <div className="flex items-center gap-3">
      <Button
        variant="outline"
        onClick={() => runScan()}
        disabled={isScanning}
      >
        {isScanning ? (
          <><Loader2 className="h-4 w-4 animate-spin" /> Scanning...</>
        ) : (
          <><Search className="h-4 w-4" /> Scan Agent Paths</>
        )}
      </Button>
      {discovered && (
        <span className="text-sm text-muted-foreground">
          {discovered.length} unmanaged skill{discovered.length !== 1 ? "s" : ""} found
        </span>
      )}
    </div>

    {/* Results grouped by scope */}
    {discovered && discovered.length > 0 && (
      <DiscoverResults
        discovered={discovered}
        skills={skills ?? []}
        onDelegate={(selected) => {
          setDelegateSelection(selected)
          setShowDelegateDialog(true)
        }}
      />
    )}

    {discovered && discovered.length === 0 && (
      <p className="text-muted-foreground text-sm pt-4">
        No unmanaged skills found in agent paths. All skills are either managed or
        no agents are configured.
      </p>
    )}
  </div>
)}
```

- [ ] **Step 4: Create DiscoverResults sub-component**

Add as a function component in the same file (or extract to a new file if it gets large). This groups results by scope and shows checkboxes for batch delegation:

```tsx
function DiscoverResults({
  discovered,
  skills,
  onDelegate,
}: {
  discovered: DiscoveredSkill[]
  skills: Skill[]
  onDelegate: (selected: DiscoveredSkill[]) => void
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set())

  // Group by scope
  const groups = useMemo(() => {
    const map = new Map<string, DiscoveredSkill[]>()
    for (const d of discovered) {
      const key = d.scope === "global" ? "Global Skills" : `Project: ${d.scope}`
      const list = map.get(key) ?? []
      list.push(d)
      map.set(key, list)
    }
    return map
  }, [discovered])

  function toggleAll() {
    if (selected.size === discovered.length) {
      setSelected(new Set())
    } else {
      setSelected(new Set(discovered.map((d) => d.found_path)))
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <button onClick={toggleAll} className="text-xs text-primary hover:underline">
          {selected.size === discovered.length ? "Deselect All" : "Select All"}
        </button>
        <Button
          size="sm"
          disabled={selected.size === 0}
          onClick={() => {
            const items = discovered.filter((d) => selected.has(d.found_path))
            onDelegate(items)
          }}
        >
          Delegate {selected.size > 0 ? `(${selected.size})` : ""}
        </Button>
      </div>

      {[...groups.entries()].map(([groupName, items]) => (
        <div key={groupName} className="space-y-2">
          <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {groupName}
          </h4>
          <div className="space-y-1.5">
            {items.map((d) => {
              const isSelected = selected.has(d.found_path)
              return (
                <button
                  key={d.found_path}
                  type="button"
                  onClick={() => {
                    setSelected((prev) => {
                      const next = new Set(prev)
                      if (next.has(d.found_path)) next.delete(d.found_path)
                      else next.add(d.found_path)
                      return next
                    })
                  }}
                  className={`flex w-full items-center gap-3 rounded-lg border p-3 text-left transition-colors ${
                    isSelected
                      ? d.exists_in_registry
                        ? "border-amber-500 bg-amber-500/5"
                        : "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/30"
                  }`}
                >
                  <div className={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                    isSelected ? "border-primary bg-primary text-primary-foreground" : "border-muted-foreground/30"
                  }`}>
                    {isSelected && <Check className="h-3 w-3" />}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{d.name}</span>
                      <Badge variant="outline" className="text-[10px]">{d.agent_name}</Badge>
                      {d.exists_in_registry && (
                        <Badge variant="secondary" className="text-[10px] text-amber-600">
                          Exists
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground truncate">
                      {d.description ?? "No description"}
                    </p>
                  </div>
                  <span className="text-xs text-muted-foreground shrink-0">
                    ~{formatTokens(d.token_estimate)} tokens
                  </span>
                </button>
              )
            })}
          </div>
        </div>
      ))}
    </div>
  )
}
```

- [ ] **Step 5: Add delegation dialog**

Add state and dialog for choosing/creating a profile when delegating:

```tsx
// In Skills() component state:
const [delegateSelection, setDelegateSelection] = useState<DiscoveredSkill[]>([])
const [showDelegateDialog, setShowDelegateDialog] = useState(false)
const [delegateMode, setDelegateMode] = useState<"existing" | "new">("new")
const [delegateProfileName, setDelegateProfileName] = useState("")
const [delegateProfileDesc, setDelegateProfileDesc] = useState("")
const [delegateExistingProfile, setDelegateExistingProfile] = useState("")

// Query for profiles
const { data: profilesData } = useQuery({
  queryKey: ["profiles"],
  queryFn: listProfiles,
})

const delegateMutation = useMutation({
  mutationFn: () => {
    const reqs: DelegateRequest[] = delegateSelection.map((d) => ({
      name: d.name,
      agent_name: d.agent_name,
      found_path: d.found_path,
    }))
    const profileName = delegateMode === "new" ? delegateProfileName : delegateExistingProfile
    return delegateSkills(
      reqs,
      profileName,
      delegateMode === "new",
      delegateMode === "new" ? delegateProfileDesc || undefined : undefined,
    )
  },
  onSuccess: (msg) => {
    toast.success(msg)
    queryClient.invalidateQueries({ queryKey: ["skills"] })
    queryClient.invalidateQueries({ queryKey: ["profiles"] })
    queryClient.invalidateQueries({ queryKey: ["discoveredSkills"] })
    closeDelegateDialog()
  },
  onError: (err) => toast.error(String(err)),
})

function closeDelegateDialog() {
  setShowDelegateDialog(false)
  setDelegateSelection([])
  setDelegateProfileName("")
  setDelegateProfileDesc("")
  setDelegateExistingProfile("")
  setDelegateMode("new")
}
```

Dialog JSX:

```tsx
{/* Delegate Dialog */}
<Dialog open={showDelegateDialog} onOpenChange={(o) => { if (!o) closeDelegateDialog() }}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Delegate {delegateSelection.length} Skill{delegateSelection.length !== 1 ? "s" : ""}</DialogTitle>
    </DialogHeader>
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Import into registry and assign to a profile for management.
      </p>

      {/* Mode toggle */}
      <div className="flex gap-1 rounded-lg bg-muted p-1">
        <button
          onClick={() => setDelegateMode("new")}
          className={`flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
            delegateMode === "new"
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          Create New Profile
        </button>
        <button
          onClick={() => setDelegateMode("existing")}
          className={`flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
            delegateMode === "existing"
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          Add to Existing
        </button>
      </div>

      {delegateMode === "new" ? (
        <>
          <div className="space-y-2">
            <Label>Profile Name</Label>
            <Input
              value={delegateProfileName}
              onChange={(e) => setDelegateProfileName(e.target.value)}
              placeholder="e.g. imported-global"
            />
          </div>
          <div className="space-y-2">
            <Label>Description (optional)</Label>
            <Input
              value={delegateProfileDesc}
              onChange={(e) => setDelegateProfileDesc(e.target.value)}
              placeholder="Skills delegated from agent paths"
            />
          </div>
        </>
      ) : (
        <div className="space-y-2">
          <Label>Select Profile</Label>
          <select
            value={delegateExistingProfile}
            onChange={(e) => setDelegateExistingProfile(e.target.value)}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm"
          >
            <option value="">Choose a profile...</option>
            {(profilesData?.profiles ?? []).map((p) => (
              <option key={p.name} value={p.name}>{p.name}</option>
            ))}
          </select>
        </div>
      )}

      {/* Selected skills summary */}
      <div className="rounded-md border border-border p-3 max-h-32 overflow-y-auto">
        {delegateSelection.map((d) => (
          <div key={d.found_path} className="flex items-center gap-2 text-sm py-0.5">
            <FileCode className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <span className="truncate">{d.name}</span>
            <Badge variant="outline" className="text-[10px] ml-auto">{d.agent_name}</Badge>
          </div>
        ))}
      </div>
    </div>
    <DialogFooter>
      <Button variant="outline" onClick={closeDelegateDialog}>Cancel</Button>
      <Button
        onClick={() => delegateMutation.mutate()}
        disabled={
          delegateMutation.isPending ||
          (delegateMode === "new" && !delegateProfileName) ||
          (delegateMode === "existing" && !delegateExistingProfile)
        }
      >
        {delegateMutation.isPending ? "Delegating..." : "Delegate"}
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

- [ ] **Step 6: TypeScript check**

Run: `cd crates/skills-gui && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add crates/skills-gui/src/pages/Skills.tsx
git commit -m "feat: add Discover tab with scan results and delegation dialog"
```

---

### Task 11: Add "Link to Remote" in skill detail panel

**Files:**
- Modify: `crates/skills-gui/src/pages/Skills.tsx`

- [ ] **Step 1: Add link remote state and mutation**

In `Skills()` component:

```typescript
const [showLinkRemote, setShowLinkRemote] = useState(false)
const [linkUrl, setLinkUrl] = useState("")
const [linkRef, setLinkRef] = useState("main")
const [linkSubpath, setLinkSubpath] = useState("")

const linkRemoteMutation = useMutation({
  mutationFn: () => linkRemote(detail!.name, linkUrl, linkRef, linkSubpath || undefined),
  onSuccess: (msg) => {
    toast.success(msg)
    queryClient.invalidateQueries({ queryKey: ["skills"] })
    setShowLinkRemote(false)
    setLinkUrl("")
    setLinkRef("main")
    setLinkSubpath("")
  },
  onError: (err) => toast.error(String(err)),
})

const unlinkRemoteMutation = useMutation({
  mutationFn: () => unlinkRemote(detail!.name),
  onSuccess: (msg) => {
    toast.success(msg)
    queryClient.invalidateQueries({ queryKey: ["skills"] })
  },
  onError: (err) => toast.error(String(err)),
})
```

Import `linkRemote`, `unlinkRemote` from `@/lib/api`.

- [ ] **Step 2: Add source action button in detail panel**

In the detail panel's Details section (after the Source/URL display), add:

```tsx
{/* Source actions */}
{detail && (
  <div className="pt-1">
    {detail.source_type === "git" ? (
      <Button
        variant="outline"
        size="sm"
        className="text-xs"
        onClick={() => unlinkRemoteMutation.mutate()}
        disabled={unlinkRemoteMutation.isPending}
      >
        {unlinkRemoteMutation.isPending ? "Unlinking..." : "Unlink Remote"}
      </Button>
    ) : (
      <Button
        variant="outline"
        size="sm"
        className="text-xs"
        onClick={() => setShowLinkRemote(true)}
      >
        <Globe className="h-3.5 w-3.5" />
        Link to Remote
      </Button>
    )}
  </div>
)}
```

- [ ] **Step 3: Add Link to Remote dialog**

```tsx
<Dialog open={showLinkRemote} onOpenChange={(o) => { if (!o) setShowLinkRemote(false) }}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Link to Remote</DialogTitle>
    </DialogHeader>
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Link &quot;{detail?.name}&quot; to a GitHub repository. This enables pulling
        updates from upstream via the update command.
      </p>
      <div className="space-y-2">
        <Label>GitHub URL</Label>
        <Input
          value={linkUrl}
          onChange={(e) => setLinkUrl(e.target.value)}
          placeholder="https://github.com/owner/repo"
        />
      </div>
      <div className="flex gap-3">
        <div className="flex-1 space-y-2">
          <Label>Git Ref</Label>
          <Input
            value={linkRef}
            onChange={(e) => setLinkRef(e.target.value)}
            placeholder="main"
          />
        </div>
        <div className="flex-1 space-y-2">
          <Label>Subpath (optional)</Label>
          <Input
            value={linkSubpath}
            onChange={(e) => setLinkSubpath(e.target.value)}
            placeholder="skills/my-skill"
          />
        </div>
      </div>
    </div>
    <DialogFooter>
      <Button variant="outline" onClick={() => setShowLinkRemote(false)}>Cancel</Button>
      <Button
        onClick={() => linkRemoteMutation.mutate()}
        disabled={!linkUrl || linkRemoteMutation.isPending}
      >
        {linkRemoteMutation.isPending ? "Linking..." : "Link"}
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

- [ ] **Step 4: Extend SkillSchema to include source_type**

The detail panel needs `source_type` to decide which button to show. Check if `source_type` is already in `SkillSchema`. If it is (as a nullable string), use it. If the Tauri `SkillInfo` struct doesn't expose it, add it.

In `crates/skills-gui/src-tauri/src/commands.rs`, ensure `SkillInfo` has:

```rust
pub source_type: Option<String>,
```

And in the `list_skills` mapping, add:

```rust
source_type: s.source.as_ref().map(|src| format!("{:?}", src.source_type).to_lowercase()),
```

Verify `SkillSchema` in `schemas.ts` already has `source_type: z.string().nullable()`. (It does based on earlier reading.)

- [ ] **Step 5: TypeScript check**

Run: `cd crates/skills-gui && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/skills-gui/src/pages/Skills.tsx crates/skills-gui/src-tauri/src/commands.rs crates/skills-gui/src/lib/schemas.ts
git commit -m "feat: add Link to Remote / Unlink Remote in skill detail panel"
```

---

## Chunk 6: Settings UI + MCP Tools

### Task 12: Add auto-scan settings to Settings page

**Files:**
- Modify: `crates/skills-gui/src/pages/Settings.tsx`
- Modify: `crates/skills-gui/src/lib/api.ts` (SettingsPayload)
- Modify: `crates/skills-gui/src-tauri/src/commands.rs` (get_settings/save_settings)

- [ ] **Step 1: Extend SettingsPayload with scan fields**

In `api.ts`, update `SettingsPayload`:

```typescript
export interface SettingsPayload {
  mcp_enabled: boolean
  mcp_port: number
  mcp_transport: string
  git_sync_enabled: boolean
  git_sync_repo_url: string
  scan_auto_on_startup: boolean
}
```

- [ ] **Step 2: Update Tauri get_settings/save_settings**

In `commands.rs`, update the settings serialization to include `scan.auto_scan_on_startup`.

- [ ] **Step 3: Add Settings UI section**

In `Settings.tsx`, after the Git Sync section, add:

```tsx
{/* Skill Discovery */}
<section className="space-y-4 rounded-xl border border-border bg-card p-6">
  <h3 className="text-base font-semibold">Skill Discovery</h3>
  <div className="flex items-center justify-between">
    <div className="space-y-0.5">
      <span className="text-sm font-medium">Auto-scan on Startup</span>
      <p className="text-xs text-muted-foreground">
        Automatically scan agent paths for unmanaged skills when the app starts
      </p>
    </div>
    <Switch
      checked={settings?.scan_auto_on_startup ?? false}
      onCheckedChange={(checked) => update({ scan_auto_on_startup: checked })}
      disabled={!settings || saving}
    />
  </div>
</section>
```

- [ ] **Step 4: Update default settings state**

In `Settings.tsx`, update the fallback state:

```typescript
setSettings({
  mcp_enabled: false,
  mcp_port: 3100,
  mcp_transport: "stdio",
  git_sync_enabled: false,
  git_sync_repo_url: "",
  scan_auto_on_startup: false,
})
```

- [ ] **Step 5: TypeScript check + build check**

Run: `cd crates/skills-gui && npx tsc --noEmit && cd ../.. && cargo check`
Expected: Both pass

- [ ] **Step 6: Commit**

```bash
git add crates/skills-gui/src/pages/Settings.tsx crates/skills-gui/src/lib/api.ts crates/skills-gui/src-tauri/src/commands.rs
git commit -m "feat: add auto-scan on startup setting"
```

---

### Task 13: Add MCP tools for discovery and linking

**Files:**
- Modify: `crates/skills-mcp/src/lib.rs`

- [ ] **Step 1: Add tool definitions**

Add 3 new tools to the `tools/list` handler:

```rust
{
    "name": "discover_skills",
    "description": "Scan agent paths for unmanaged skills not tracked by skills-mgr",
    "inputSchema": {
        "type": "object",
        "properties": {
            "global_only": { "type": "boolean", "description": "Only scan global paths" }
        }
    }
}
```

```rust
{
    "name": "link_remote",
    "description": "Link a local skill to a remote GitHub URL for upstream sync",
    "inputSchema": {
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "url": { "type": "string" },
            "subpath": { "type": "string" },
            "git_ref": { "type": "string", "default": "main" }
        },
        "required": ["name", "url"]
    }
}
```

```rust
{
    "name": "unlink_remote",
    "description": "Unlink a skill from its remote URL, reverting to local type",
    "inputSchema": {
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"]
    }
}
```

- [ ] **Step 2: Add tool call handlers**

In the `tools/call` match arms:

```rust
"discover_skills" => {
    let global_only = params.get("global_only").and_then(|v| v.as_bool()).unwrap_or(false);
    let agents_config = AgentsConfig::load(&dirs.agents_toml())?;
    let project_paths = if global_only {
        vec![]
    } else {
        db.list_all_projects().await?
            .into_iter()
            .filter(|p| p.path != "__global__")
            .map(|p| p.path)
            .collect()
    };
    let discovered = skills_core::discovery::scan_all_agents(
        &dirs, &registry, &agents_config, &project_paths
    )?;
    let result: Vec<serde_json::Value> = discovered.iter().map(|d| {
        serde_json::json!({
            "name": d.name,
            "description": d.description,
            "agent_name": d.agent_name,
            "found_path": d.found_path.to_string_lossy(),
            "scope": match &d.scope {
                skills_core::discovery::DiscoveryScope::Global => "global".to_string(),
                skills_core::discovery::DiscoveryScope::Project(p) => p.clone(),
            },
            "files": d.files.len(),
            "token_estimate": d.token_estimate,
            "exists_in_registry": d.exists_in_registry,
        })
    }).collect();
    serde_json::to_string_pretty(&result)?
}
```

```rust
"link_remote" => {
    let name = params["name"].as_str().context("name required")?;
    let url = params["url"].as_str().context("url required")?;
    let subpath = params.get("subpath").and_then(|v| v.as_str());
    let git_ref = params.get("git_ref").and_then(|v| v.as_str()).unwrap_or("main");
    registry.link_remote(name, url, subpath, git_ref)?;
    format!("Linked '{}' to remote: {} (ref: {})", name, url, git_ref)
}
```

```rust
"unlink_remote" => {
    let name = params["name"].as_str().context("name required")?;
    registry.unlink_remote(name)?;
    format!("Unlinked '{}' from remote", name)
}
```

- [ ] **Step 3: Verify build**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add crates/skills-mcp/src/lib.rs
git commit -m "feat: add MCP tools for discover_skills, link_remote, unlink_remote"
```

---

### Task 14: Final verification and integration commit

- [ ] **Step 1: Run full Rust test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 2: Run TypeScript check**

Run: `cd crates/skills-gui && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Run Rust lint**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Manual smoke test**

1. `cargo run -p skills-cli -- skill discover` — should list unmanaged skills or report none
2. Build GUI with `cd crates/skills-gui && pnpm tauri dev` — verify Skills page has Discover tab
3. Click "Scan Agent Paths" — verify scan runs and shows results
4. Test "Link to Remote" on any local skill in detail panel

- [ ] **Step 5: Create feature branch and PR**

```bash
git checkout -b feat/skill-delegation
git push -u origin feat/skill-delegation
gh pr create --base master --title "feat: skill delegation and remote linking" --body "..."
```
