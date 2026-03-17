use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::{AgentsConfig, AppDirs, SourcesConfig};
use crate::placements;
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
    Project(String),
}

/// Scan a single agent path (global or project) for skills not managed by skills-mgr.
///
/// Returns discovered skills. Skills already in the registry are flagged with
/// `exists_in_registry = true` but still returned (for conflict display).
/// `placed_paths` contains target paths of skills placed by skills-mgr
/// (from the placements DB). Skills at these paths are skipped.
pub fn scan_agent_path(
    registry: &Registry,
    sources: &SourcesConfig,
    placed_paths: &HashSet<String>,
    agent_name: &str,
    scan_path: &Path,
    scope: DiscoveryScope,
) -> Result<Vec<DiscoveredSkill>> {
    if !scan_path.exists() {
        return Ok(vec![]);
    }
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

        // Canonicalize for reliable comparison (e.g. /var → /private/var on macOS)
        let canonical = std::fs::canonicalize(&skill_dir).unwrap_or_else(|_| skill_dir.clone());
        let canonical_str = canonical.to_string_lossy();

        // Skip skills placed by skills-mgr (via profile activation)
        if placed_paths.contains(&*canonical_str) {
            continue;
        }

        // Skip if this skill was previously delegated (tracked in sources.toml)
        let is_tracked_delegation = sources
            .skills
            .values()
            .any(|s| s.original_agent_path.as_deref() == Some(&*canonical_str));
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

/// Scan all configured agent paths for unmanaged skills.
///
/// Scans each enabled agent's global_path and, for each provided project path,
/// the agent's project_path resolved relative to that project.
///
/// `project_paths` is a list of known project directories to scan for
/// project-level skills. Pass empty slice to scan only global paths.
/// `placed_paths` contains target paths of all skills placed by skills-mgr
/// (queried from the placements DB by the caller). Skills at these paths
/// are excluded from discovery results.
pub fn scan_all_agents(
    dirs: &AppDirs,
    registry: &Registry,
    agents_config: &AgentsConfig,
    project_paths: &[String],
    placed_paths: &HashSet<String>,
) -> Result<Vec<DiscoveredSkill>> {
    let sources = match SourcesConfig::load(&dirs.sources_toml()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("could not load sources.toml, treating all skills as unmanaged: {e}");
            SourcesConfig::default()
        }
    };
    let mut all = Vec::new();

    for (agent_name, agent_def) in &agents_config.agents {
        if !agent_def.enabled {
            continue;
        }

        // Scan global path
        let global_expanded = placements::expand_tilde(&agent_def.global_path);
        let global_path = PathBuf::from(&global_expanded);
        let global_results = scan_agent_path(
            registry,
            &sources,
            placed_paths,
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
                registry,
                &sources,
                placed_paths,
                agent_name,
                &agent_project_dir,
                DiscoveryScope::Project(project_path.clone()),
            )?;
            all.extend(project_results);
        }
    }

    Ok(all)
}

/// Parse description from a SKILL.md frontmatter (delegates to shared parser).
fn parse_skill_description(skill_md: &Path) -> Option<String> {
    crate::frontmatter::parse_description(skill_md)
}

/// List files and compute size stats for a skill directory.
/// Lightweight version that doesn't error — returns empty on failure.
fn list_skill_files(dir: &Path) -> (Vec<String>, u64, u64) {
    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    collect_files(dir, dir, &mut files, &mut total_bytes, 0);
    files.sort();
    // ~4 bytes per token — rough ASCII approximation, used for display only
    (files, total_bytes, total_bytes / 4)
}

const MAX_COLLECT_DEPTH: u32 = 10;

fn collect_files(
    base: &Path,
    current: &Path,
    files: &mut Vec<String>,
    total_bytes: &mut u64,
    depth: u32,
) {
    if depth > MAX_COLLECT_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // DirEntry::file_type() does not follow symlinks, so is_dir() is already
        // false for symlinks-to-directories. The !is_symlink() guard is kept as
        // defense-in-depth against platform differences.
        if entry
            .file_type()
            .map(|t| t.is_dir() && !t.is_symlink())
            .unwrap_or(false)
        {
            collect_files(base, &path, files, total_bytes, depth + 1);
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
        let (tmp, _dirs, reg) = setup_test_env();
        let sources = SourcesConfig::default();

        let global_dir = tmp.path().join("agent-global");
        let skill_dir = global_dir.join("external-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: external-skill\ndescription: Found externally\n---\nContent here",
        )
        .unwrap();

        let results = scan_agent_path(
            &reg,
            &sources,
            &HashSet::new(),
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "external-skill");
        assert_eq!(results[0].description, Some("Found externally".into()));
        assert_eq!(results[0].agent_name, "test-agent");
        assert_eq!(results[0].scope, DiscoveryScope::Global);
        assert!(!results[0].exists_in_registry);
    }

    #[test]
    fn test_scan_global_flags_managed_skills_as_conflict() {
        let (tmp, _dirs, reg) = setup_test_env();
        let sources = SourcesConfig::default();

        reg.create("managed-skill", "Managed by us").unwrap();

        let global_dir = tmp.path().join("agent-global");
        let skill_dir = global_dir.join("managed-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: managed-skill\ndescription: Managed\n---\n",
        )
        .unwrap();

        let results = scan_agent_path(
            &reg,
            &sources,
            &HashSet::new(),
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].exists_in_registry);
    }

    #[test]
    fn test_scan_skips_dirs_without_skill_md() {
        let (tmp, _dirs, reg) = setup_test_env();
        let sources = SourcesConfig::default();

        let global_dir = tmp.path().join("agent-global");
        let random_dir = global_dir.join("not-a-skill");
        std::fs::create_dir_all(&random_dir).unwrap();
        std::fs::write(random_dir.join("README.md"), "not a skill").unwrap();

        let results = scan_agent_path(
            &reg,
            &sources,
            &HashSet::new(),
            "test-agent",
            &global_dir,
            DiscoveryScope::Global,
        )
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_nonexistent_path_returns_empty() {
        let (_tmp, _dirs, reg) = setup_test_env();
        let sources = SourcesConfig::default();

        let results = scan_agent_path(
            &reg,
            &sources,
            &HashSet::new(),
            "test-agent",
            Path::new("/nonexistent/path"),
            DiscoveryScope::Global,
        )
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_all_agents_global() {
        let (tmp, dirs, reg) = setup_test_env();

        let global_dir = tmp.path().join("claude-global");
        let skill_dir = global_dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Test\n---\n",
        )
        .unwrap();

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

        let results = scan_all_agents(&dirs, &reg, &agents_config, &[], &HashSet::new()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my-skill");
        assert_eq!(results[0].agent_name, "claude");
        assert_eq!(results[0].scope, DiscoveryScope::Global);
    }

    #[test]
    fn test_parse_yaml_multiline_description() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: >\n  Creates Python projects\n  with proper structure.\n---\n",
        )
        .unwrap();

        let desc = parse_skill_description(&skill_md);
        assert_eq!(
            desc,
            Some("Creates Python projects with proper structure.".into())
        );
    }

    #[test]
    fn test_parse_inline_description() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: Inline desc\n---\n",
        )
        .unwrap();

        let desc = parse_skill_description(&skill_md);
        assert_eq!(desc, Some("Inline desc".into()));
    }
}
