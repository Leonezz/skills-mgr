use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    pub fn base(&self) -> &Path {
        &self.base
    }
    pub fn registry(&self) -> PathBuf {
        self.base.join("registry")
    }
    pub fn sources_toml(&self) -> PathBuf {
        self.base.join("sources.toml")
    }
    pub fn profiles_toml(&self) -> PathBuf {
        self.base.join("profiles.toml")
    }
    pub fn agents_toml(&self) -> PathBuf {
        self.base.join("agents.toml")
    }
    pub fn local(&self) -> PathBuf {
        self.base.join("local")
    }
    pub fn database(&self) -> PathBuf {
        self.base.join("local").join("skills-mgr.db")
    }
    pub fn cache(&self) -> PathBuf {
        self.base.join("local").join("cache")
    }
    pub fn settings_toml(&self) -> PathBuf {
        self.base.join("settings.toml")
    }

    /// Ensure all required directories exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.registry())?;
        std::fs::create_dir_all(self.local())?;
        std::fs::create_dir_all(self.cache())?;
        ensure_teaching_skill(self)?;
        Ok(())
    }
}

/// Write the built-in teaching skill to the registry if not already present.
fn ensure_teaching_skill(dirs: &AppDirs) -> Result<()> {
    let skill_dir = dirs.registry().join("skills-mgr-guide");
    if skill_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        include_str!("../assets/skills-mgr-guide.md"),
    )?;
    Ok(())
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
    #[serde(default)]
    pub original_agent_path: Option<String>,
    /// Provider type that sourced this skill (e.g., "github", "hub", "feed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Hub name for hub-sourced skills (e.g., "clawhub").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_name: Option<String>,
    /// Hub-specific skill identifier (e.g., slug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_skill_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Git,
    Registry,
    Local,
    Hub,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesConfig {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub profiles: std::collections::BTreeMap<String, ProfileDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub skills: Vec<String>,
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
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl SourcesConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
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
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub mcp: McpSettings,
    #[serde(default)]
    pub git_sync: GitSyncSettings,
    #[serde(default)]
    pub scan: ScanSettings,
    #[serde(default)]
    pub hubs: Vec<HubConfig>,
}

/// Configuration for a skill hub — a web-based registry or featured feed
/// that provides skill discovery and download outside of direct GitHub URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubConfig {
    /// Unique identifier (e.g., "clawhub").
    pub name: String,
    /// Human-readable label for UI display (e.g., "ClawHub").
    pub display_name: String,
    /// How this hub serves skills.
    pub hub_type: HubType,
    /// Base URL — for feeds this is the JSON URL, for APIs this is the base endpoint.
    pub base_url: String,
    /// Whether this hub is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Environment variable name containing an API key (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Page URL prefix for matching skill URLs (e.g., "https://clawhub.ai").
    /// When a user pastes a URL starting with this prefix, the hub provider claims it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    /// Download API URL template. `{slug}` is replaced with the skill slug.
    /// Example: "https://wry-manatee-359.convex.site/api/v1/download?slug={slug}"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_api_url: Option<String>,
}

/// How a skill hub serves its content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HubType {
    /// A JSON file listing skills with `source_url` fields pointing to actual content.
    /// Schema: `{ skills: [{ slug, name, summary, source_url, ... }] }`
    Feed,
    /// A REST API for browsing and downloading skills directly.
    Api,
}

/// Merge built-in hubs with user-configured hubs from settings.
/// User hubs override built-ins by name; extras are appended.
pub fn merge_hubs(settings_path: &Path) -> Vec<HubConfig> {
    let mut hubs = builtin_hubs();
    if let Ok(settings) = AppSettings::load(settings_path) {
        for user_hub in settings.hubs {
            if let Some(pos) = hubs.iter().position(|h| h.name == user_hub.name) {
                hubs[pos] = user_hub;
            } else {
                hubs.push(user_hub);
            }
        }
    }
    hubs
}

/// Built-in hub definitions that are always available.
/// Users can override these or add custom hubs in settings.toml.
pub fn builtin_hubs() -> Vec<HubConfig> {
    vec![HubConfig {
        name: "clawhub".to_string(),
        display_name: "ClawHub".to_string(),
        hub_type: HubType::Feed,
        base_url:
            "https://raw.githubusercontent.com/qufei1993/skills-hub/main/featured-skills.json"
                .to_string(),
        enabled: true,
        api_key_env: None,
        page_url: Some("https://clawhub.ai".to_string()),
        download_api_url: Some(
            "https://wry-manatee-359.convex.site/api/v1/download?slug={slug}".to_string(),
        ),
    }]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanSettings {
    #[serde(default)]
    pub auto_scan_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_mcp_transport")]
    pub transport: String,
}

fn default_mcp_port() -> u16 {
    3100
}

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_mcp_port(),
            transport: default_mcp_transport(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitSyncSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub repo_url: String,
}

impl AppSettings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
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
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
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
            global: GlobalConfig::default(),
            base: BaseConfig {
                skills: vec!["code-review".into(), "obsidian".into()],
            },
            profiles: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "rust".into(),
                    ProfileDef {
                        description: Some("Rust development".into()),
                        skills: vec!["rust-engineer".into()],
                        includes: vec![],
                    },
                );
                m.insert(
                    "rust-react".into(),
                    ProfileDef {
                        description: Some("Full-stack".into()),
                        skills: vec!["api-design".into()],
                        includes: vec!["rust".into()],
                    },
                );
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
                m.insert(
                    "rust-engineer".into(),
                    SkillSource {
                        source_type: SourceType::Git,
                        url: Some("https://github.com/anthropics/skills".into()),
                        path: Some("rust-engineer".into()),
                        git_ref: Some("main".into()),
                        hash: Some("sha256:abc123".into()),
                        updated_at: Some("2026-03-10T12:00:00Z".into()),
                        original_agent_path: None,
                        provider: Some("github".into()),
                        hub_name: None,
                        hub_skill_id: None,
                    },
                );
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
                m.insert(
                    "claude-code".into(),
                    AgentDef {
                        project_path: ".claude/skills".into(),
                        global_path: "~/.claude/skills".into(),
                        enabled: true,
                    },
                );
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
                        provider: None,
                        hub_name: None,
                        hub_skill_id: None,
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
            hubs: vec![],
        };
        settings.save(&path).unwrap();
        let loaded = AppSettings::load(&path).unwrap();
        assert!(loaded.scan.auto_scan_on_startup);
    }

    #[test]
    fn test_hub_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.toml");

        let settings = AppSettings {
            hubs: vec![HubConfig {
                name: "test-hub".into(),
                display_name: "Test Hub".into(),
                hub_type: HubType::Feed,
                base_url: "https://example.com/feed.json".into(),
                enabled: true,
                api_key_env: None,
                page_url: None,
                download_api_url: None,
            }],
            ..Default::default()
        };
        settings.save(&path).unwrap();
        let loaded = AppSettings::load(&path).unwrap();
        assert_eq!(loaded.hubs.len(), 1);
        assert_eq!(loaded.hubs[0].name, "test-hub");
        assert_eq!(loaded.hubs[0].hub_type, HubType::Feed);
    }

    #[test]
    fn test_builtin_hubs() {
        let hubs = builtin_hubs();
        assert!(!hubs.is_empty());
        assert_eq!(hubs[0].name, "clawhub");
        assert!(hubs[0].enabled);
    }
}
