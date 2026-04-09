use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::config::{AppDirs, SkillSource, SourceType, SourcesConfig};
use crate::remote;

/// Metadata parsed from a SKILL.md frontmatter.
#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: Option<String>,
    pub dir_path: PathBuf,
    pub files: Vec<String>,
    pub source: Option<SkillSource>,
    pub total_bytes: u64,
    pub token_estimate: u64,
    /// Token cost of just name + description (startup cost per conversation).
    pub metadata_token_estimate: u64,
}

/// Result of updating a single skill from its remote source.
#[derive(Debug)]
pub enum SkillUpdateResult {
    Updated {
        name: String,
        old_hash: String,
        new_hash: String,
    },
    AlreadyUpToDate {
        name: String,
    },
    Skipped {
        name: String,
        reason: String,
    },
    Failed {
        name: String,
        error: String,
    },
}

/// Manages the skill registry directory.
pub struct Registry {
    pub(crate) dirs: AppDirs,
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
            let (files, total_bytes, token_estimate) = list_files_with_stats(&skill_dir)?;
            let source = sources.skills.get(&name).cloned();
            let metadata_token_estimate = compute_metadata_tokens(&name, description.as_deref());

            skills.push(SkillMeta {
                name,
                description,
                dir_path: skill_dir,
                files,
                source,
                total_bytes,
                token_estimate,
                metadata_token_estimate,
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
        let (files, total_bytes, token_estimate) = list_files_with_stats(&skill_dir)?;
        let source = sources.skills.get(name).cloned();
        let metadata_token_estimate = compute_metadata_tokens(name, description.as_deref());

        Ok(Some(SkillMeta {
            name: name.to_string(),
            description,
            dir_path: skill_dir,
            files,
            source,
            total_bytes,
            token_estimate,
            metadata_token_estimate,
        }))
    }

    /// Check if a skill exists in the registry.
    pub fn exists(&self, name: &str) -> bool {
        self.dirs.registry().join(name).join("SKILL.md").exists()
    }

    /// Create a new skill with a scaffold SKILL.md.
    pub fn create(&self, name: &str, description: &str) -> Result<PathBuf> {
        if name.contains('/') || name.contains('\\') || name.contains(':') {
            bail!(
                "Invalid skill name '{}': must not contain '/', '\\', or ':'",
                name
            );
        }
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
        sources.skills.insert(
            name.to_string(),
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
                original_agent_path: None,
                provider: None,
                hub_name: None,
                hub_skill_id: None,
            },
        );
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

    /// Update a skill's SKILL.md description.
    pub fn update_description(&self, name: &str, description: &str) -> Result<()> {
        let skill_dir = self.dirs.registry().join(name);
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            bail!("Skill '{}' not found in registry", name);
        }
        let content = std::fs::read_to_string(&skill_md)?;
        let updated = update_frontmatter_description(&content, description);
        std::fs::write(&skill_md, updated)?;
        Ok(())
    }

    /// Read the raw SKILL.md content.
    pub fn read_content(&self, name: &str) -> Result<String> {
        let skill_md = self.dirs.registry().join(name).join("SKILL.md");
        if !skill_md.exists() {
            bail!("Skill '{}' not found in registry", name);
        }
        Ok(std::fs::read_to_string(&skill_md)?)
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
                original_agent_path: None,
                provider: None,
                hub_name: None,
                hub_skill_id: None,
            },
        );
        sources.save(&self.dirs.sources_toml())?;

        Ok(name)
    }

    /// Import a skill from an already-extracted directory (e.g. staging).
    ///
    /// Copies the skill directory into the registry and records source metadata.
    pub fn import_from_extracted_dir(
        &self,
        source_dir: &Path,
        skill_name: &str,
        owner: &str,
        repo: &str,
        git_ref: &str,
        subpath: &str,
    ) -> Result<()> {
        let skill_md = source_dir.join("SKILL.md");
        if !skill_md.exists() {
            bail!("No SKILL.md found at {}", source_dir.display());
        }

        let dest = self.dirs.registry().join(skill_name);
        if dest.exists() {
            bail!("Skill '{}' already exists in registry", skill_name);
        }

        copy_dir_recursive(source_dir, &dest)?;

        let hash = compute_tree_hash(&dest)?;
        let canonical = format!(
            "https://github.com/{}/{}/tree/{}/{}",
            owner, repo, git_ref, subpath
        );

        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.insert(
            skill_name.to_string(),
            SkillSource {
                source_type: SourceType::Git,
                url: Some(canonical),
                path: Some(subpath.to_string()),
                git_ref: Some(git_ref.to_string()),
                hash: Some(hash),
                updated_at: Some(
                    chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                        .to_string(),
                ),
                original_agent_path: None,
                provider: Some("github".to_string()),
                hub_name: None,
                hub_skill_id: None,
            },
        );
        sources.save(&self.dirs.sources_toml())?;

        Ok(())
    }

    /// Import a skill from an extracted directory using a pre-built [`SkillSource`].
    ///
    /// This is the provider-agnostic version of [`import_from_extracted_dir`].
    /// The caller (typically a Tauri command) reads [`StagingMeta`], looks up the
    /// provider, calls `build_skill_source`, and passes the result here.
    pub fn import_with_source(
        &self,
        source_dir: &Path,
        skill_name: &str,
        skill_source: SkillSource,
    ) -> Result<()> {
        let skill_md = source_dir.join("SKILL.md");
        if !skill_md.exists() {
            bail!("No SKILL.md found at {}", source_dir.display());
        }

        let dest = self.dirs.registry().join(skill_name);
        if dest.exists() {
            bail!("Skill '{}' already exists in registry", skill_name);
        }

        copy_dir_recursive(source_dir, &dest)?;

        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.insert(skill_name.to_string(), skill_source);
        sources.save(&self.dirs.sources_toml())?;

        Ok(())
    }

    /// List available skills in a remote GitHub repo or collection.
    ///
    /// Downloads the repo and scans for SKILL.md files in subdirectories.
    pub async fn list_remote_skills(
        &self,
        url_or_shorthand: &str,
    ) -> Result<Vec<remote::RemoteSkillEntry>> {
        let source = remote::parse_github_url(url_or_shorthand)?;
        tracing::info!(
            owner = %source.owner,
            repo = %source.repo,
            "Listing remote skills"
        );
        remote::list_remote_skills(&source).await
    }

    /// Add a skill from a remote GitHub URL or shorthand.
    ///
    /// Parses the URL, downloads the tarball, extracts the skill directory,
    /// copies it to the registry, and records the source metadata.
    pub async fn add_from_remote(&self, url_or_shorthand: &str) -> Result<String> {
        let source = remote::parse_github_url(url_or_shorthand)?;
        let skill_name = remote::derive_skill_name(&source);
        tracing::info!(
            owner = %source.owner,
            repo = %source.repo,
            git_ref = %source.git_ref,
            subpath = ?source.subpath,
            skill_name = %skill_name,
            "Remote import: parsed source"
        );

        let dest = self.dirs.registry().join(&skill_name);
        if dest.exists() {
            bail!("Skill '{}' already exists in registry", skill_name);
        }

        tracing::info!("Downloading tarball...");
        let (_tmp_dir, skill_dir) = remote::download_github_skill(&source).await?;
        tracing::info!(dest = %dest.display(), "Downloaded, copying to registry");

        copy_dir_recursive(&skill_dir, &dest)?;

        let hash = compute_tree_hash(&dest)?;
        let canonical = remote::canonical_url(&source);

        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.insert(
            skill_name.clone(),
            SkillSource {
                source_type: SourceType::Git,
                url: Some(canonical),
                path: source.subpath.clone(),
                git_ref: Some(source.git_ref.clone()),
                hash: Some(hash),
                updated_at: Some(
                    chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                        .to_string(),
                ),
                original_agent_path: None,
                provider: Some("github".to_string()),
                hub_name: None,
                hub_skill_id: None,
            },
        );
        sources.save(&self.dirs.sources_toml())?;

        Ok(skill_name)
    }

    /// Add a skill from any supported remote source using a provider.
    ///
    /// The provider handles download; this method copies to registry and records metadata.
    pub async fn add_from_provider(
        &self,
        input: &str,
        provider: &dyn crate::provider::SkillProvider,
    ) -> Result<String> {
        let skill_name = provider.derive_name(input);
        tracing::info!(
            provider = provider.provider_type(),
            skill_name = %skill_name,
            input = %input,
            "Provider import: downloading"
        );

        let dest = self.dirs.registry().join(&skill_name);
        if dest.exists() {
            bail!("Skill '{}' already exists in registry", skill_name);
        }

        let (_tmp_dir, skill_dir) = provider.download_skill(input).await?;
        tracing::info!(dest = %dest.display(), "Downloaded, copying to registry");

        copy_dir_recursive(&skill_dir, &dest)?;

        let hash = compute_tree_hash(&dest)?;

        // Let the provider build its own staging meta with the data it needs
        let meta = provider.build_import_meta(input)?;
        let mut skill_source = provider.build_skill_source(&meta, &skill_name, &skill_name, &hash);
        // Ensure the URL is set to the original input if the provider didn't set one
        if skill_source.url.is_none() {
            skill_source.url = Some(input.to_string());
        }

        let mut sources = SourcesConfig::load(&self.dirs.sources_toml()).unwrap_or_default();
        sources.skills.insert(skill_name.clone(), skill_source);
        sources.save(&self.dirs.sources_toml())?;

        Ok(skill_name)
    }

    /// Import a discovered skill into the registry (delegation).
    ///
    /// Copies the skill directory into the registry and records the original
    /// agent path in sources.toml for tracking. The original skill at the
    /// source path is intentionally left in place — future scans will skip
    /// it via the `original_agent_path` tracking.
    pub fn delegate(&self, source_dir: &Path, original_path: &str) -> Result<String> {
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
                original_agent_path: Some(
                    std::fs::canonicalize(original_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| original_path.to_string()),
                ),
                provider: None,
                hub_name: None,
                hub_skill_id: None,
            },
        );
        sources.save(&self.dirs.sources_toml())?;

        Ok(name)
    }

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
        let entry = sources.skills.get_mut(name).ok_or_else(|| {
            anyhow::anyhow!(
                "Skill '{}' has no sources entry — try re-importing it first",
                name
            )
        })?;

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
        let entry = sources.skills.get_mut(name).ok_or_else(|| {
            anyhow::anyhow!(
                "Skill '{}' has no sources entry — try re-importing it first",
                name
            )
        })?;
        entry.source_type = SourceType::Local;
        entry.url = None;
        entry.path = None;
        entry.git_ref = None;
        entry.provider = None;
        entry.hub_name = None;
        entry.hub_skill_id = None;
        entry.updated_at = Some(
            chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
        );

        sources.save(&self.dirs.sources_toml())?;
        Ok(())
    }

    /// Update a Git-sourced skill from its tracked remote.
    ///
    /// Downloads a fresh tarball, compares the hash, and overwrites the
    /// registry entry if content has changed.
    pub async fn update_from_remote(
        &self,
        name: &str,
        providers: Option<&crate::ProviderRegistry>,
    ) -> Result<SkillUpdateResult> {
        let sources = SourcesConfig::load(&self.dirs.sources_toml())
            .context("Failed to load sources.toml — file may be corrupted")?;
        let source = match sources.skills.get(name) {
            Some(s) => s,
            None => {
                return Ok(SkillUpdateResult::Skipped {
                    name: name.to_string(),
                    reason: "not tracked in sources.toml".to_string(),
                });
            }
        };

        match source.source_type {
            SourceType::Git => self.update_git_skill(name, source).await,
            SourceType::Hub => self.update_hub_skill(name, source, providers).await,
            _ => Ok(SkillUpdateResult::Skipped {
                name: name.to_string(),
                reason: format!("{:?} source (not syncable)", source.source_type),
            }),
        }
    }

    async fn update_git_skill(
        &self,
        name: &str,
        source: &SkillSource,
    ) -> Result<SkillUpdateResult> {
        let url = match &source.url {
            Some(u) => u.clone(),
            None => {
                return Ok(SkillUpdateResult::Skipped {
                    name: name.to_string(),
                    reason: "no URL in source entry".to_string(),
                });
            }
        };

        let github_source = remote::parse_github_url(&url)?;
        let (_tmp_dir, skill_dir) = remote::download_github_skill(&github_source).await?;
        self.apply_update(name, &skill_dir).await
    }

    async fn update_hub_skill(
        &self,
        name: &str,
        source: &SkillSource,
        providers: Option<&crate::ProviderRegistry>,
    ) -> Result<SkillUpdateResult> {
        let providers = match providers {
            Some(p) => p,
            None => {
                return Ok(SkillUpdateResult::Skipped {
                    name: name.to_string(),
                    reason: "no provider registry available for hub sync".to_string(),
                });
            }
        };

        let provider_type = source.provider.as_deref().unwrap_or("feed");
        let provider = match providers.by_type(provider_type) {
            Some(p) => p,
            None => {
                return Ok(SkillUpdateResult::Skipped {
                    name: name.to_string(),
                    reason: format!("provider '{}' not found", provider_type),
                });
            }
        };

        let result = provider.download_for_update(source).await?;
        match result {
            Some((_tmp_dir, skill_dir)) => self.apply_update(name, &skill_dir).await,
            None => Ok(SkillUpdateResult::Skipped {
                name: name.to_string(),
                reason: format!(
                    "provider '{}' declined update — source metadata may be incomplete \
                     (hub_name={}, hub_skill_id={})",
                    provider_type,
                    source.hub_name.as_deref().unwrap_or("<none>"),
                    source.hub_skill_id.as_deref().unwrap_or("<none>"),
                ),
            }),
        }
    }

    /// Shared logic: compare hashes, replace if changed, update sources.toml.
    ///
    /// Loads sources.toml once to read the old hash and (if changed) update it.
    async fn apply_update(&self, name: &str, skill_dir: &Path) -> Result<SkillUpdateResult> {
        let new_hash = compute_tree_hash(skill_dir)?;

        let mut sources = SourcesConfig::load(&self.dirs.sources_toml())
            .context("Failed to load sources.toml — file may be corrupted")?;
        let old_hash = sources
            .skills
            .get(name)
            .and_then(|s| s.hash.clone())
            .unwrap_or_default();

        if new_hash == old_hash {
            return Ok(SkillUpdateResult::AlreadyUpToDate {
                name: name.to_string(),
            });
        }

        // Replace registry entry
        let dest = self.dirs.registry().join(name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        copy_dir_recursive(skill_dir, &dest)?;

        // Update hash in the already-loaded sources config
        if let Some(entry) = sources.skills.get_mut(name) {
            entry.hash = Some(new_hash.clone());
            entry.updated_at = Some(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string(),
            );
        }
        sources.save(&self.dirs.sources_toml())?;

        Ok(SkillUpdateResult::Updated {
            name: name.to_string(),
            old_hash,
            new_hash,
        })
    }

    /// Update all remote-sourced skills (Git and Hub) from their tracked remotes.
    pub async fn sync_all(
        &self,
        providers: Option<&crate::ProviderRegistry>,
    ) -> Result<Vec<SkillUpdateResult>> {
        let sources = SourcesConfig::load(&self.dirs.sources_toml())
            .context("Failed to load sources.toml — file may be corrupted")?;
        let syncable_skills: Vec<String> = sources
            .skills
            .iter()
            .filter(|(_, s)| s.source_type == SourceType::Git || s.source_type == SourceType::Hub)
            .map(|(name, _)| name.clone())
            .collect();

        let mut results = Vec::new();
        for name in &syncable_skills {
            match self.update_from_remote(name, providers).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(SkillUpdateResult::Failed {
                    name: name.clone(),
                    error: e.to_string(),
                }),
            }
        }
        Ok(results)
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

/// Rough approximation: one token ~ 4 bytes of UTF-8 text/code.
const BYTES_PER_TOKEN: u64 = 4;

/// Known binary file extensions to exclude from token estimation.
const BINARY_EXTENSIONS: &[&str] = &[
    // images
    "png", "jpg", "jpeg", "gif", "svg", "ico", "webp", "bmp", // archives / compiled
    "tar", "gz", "zip", "wasm", "bin", "exe", "dll", "so", "dylib", "o", "a", // fonts
    "ttf", "otf", "woff", "woff2", "eot", // media
    "mp3", "mp4", "mov", "avi", "mkv", "webm", "flac", "wav", // documents / databases
    "pdf", "db", "sqlite", "sqlite3", // generated lockfiles
    "lock",
];

pub fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| !BINARY_EXTENSIONS.iter().any(|&b| b.eq_ignore_ascii_case(e)))
        .unwrap_or(true)
}

/// Compute the token cost of just name + description (startup cost per conversation).
fn compute_metadata_tokens(name: &str, description: Option<&str>) -> u64 {
    let bytes = name.len() as u64 + description.unwrap_or("").len() as u64;
    bytes / BYTES_PER_TOKEN
}

/// List all files in a directory recursively, returning relative paths (sorted),
/// total text bytes, and estimated token count in a single traversal.
fn list_files_with_stats(dir: &Path) -> Result<(Vec<String>, u64, u64)> {
    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    list_files_inner(dir, dir, &mut files, Some(&mut total_bytes))?;
    files.sort();
    Ok((files, total_bytes, total_bytes / BYTES_PER_TOKEN))
}

/// List all files in a directory recursively, returning relative paths sorted.
/// Used by compute_tree_hash which only needs file paths (skips metadata).
fn list_files_recursive(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    list_files_inner(dir, dir, &mut files, None)?;
    files.sort();
    Ok(files)
}

fn list_files_inner(
    base: &Path,
    current: &Path,
    files: &mut Vec<String>,
    mut total_bytes: Option<&mut u64>,
) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            list_files_inner(base, &path, files, total_bytes.as_deref_mut())?;
        } else {
            if let Some(bytes) = total_bytes.as_deref_mut()
                && is_text_file(&path)
            {
                match entry.metadata() {
                    Ok(meta) => *bytes += meta.len(),
                    Err(e) => {
                        tracing::warn!("could not read metadata for {}: {}", path.display(), e)
                    }
                }
            }
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

/// Replace or insert the description field in SKILL.md frontmatter.
fn update_frontmatter_description(content: &str, new_description: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        // No frontmatter — prepend one
        return format!("---\ndescription: {}\n---\n\n{}", new_description, content);
    }
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("---") {
        let frontmatter = &after_first[..end];
        let rest = &after_first[end + 3..];

        let mut found = false;
        let updated_lines: Vec<String> = frontmatter
            .lines()
            .map(|line| {
                if line.trim().starts_with("description:") {
                    found = true;
                    format!("description: {}", new_description)
                } else {
                    line.to_string()
                }
            })
            .collect();

        let mut fm = updated_lines.join("\n");
        if !found {
            fm.push_str(&format!("\ndescription: {}", new_description));
        }

        format!("---{}---{}", fm, rest)
    } else {
        content.to_string()
    }
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
        let initial_count = reg.list().unwrap().len();
        reg.create("my-skill", "A test skill").unwrap();
        let skills = reg.list().unwrap();
        assert_eq!(skills.len(), initial_count + 1);
        let skill = skills.iter().find(|s| s.name == "my-skill").unwrap();
        assert_eq!(skill.description.as_deref(), Some("A test skill"));
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
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: imported-skill\ndescription: Imported\n---\nContent",
        )
        .unwrap();

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
        assert_eq!(
            std::fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn test_delegate_skill() {
        let tmp_src = TempDir::new().unwrap();
        let skill_dir = tmp_src.path().join("ext-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: ext-skill\ndescription: External\n---\nContent",
        )
        .unwrap();

        let (_tmp, reg) = setup_test_registry();
        let name = reg
            .delegate(&skill_dir, "~/.claude/skills/ext-skill")
            .unwrap();
        assert_eq!(name, "ext-skill");
        assert!(reg.exists("ext-skill"));

        let sources = SourcesConfig::load(&reg.dirs.sources_toml()).unwrap();
        let src = &sources.skills["ext-skill"];
        assert_eq!(src.source_type, SourceType::Local);
        assert_eq!(
            src.original_agent_path,
            Some("~/.claude/skills/ext-skill".into())
        );
    }

    #[test]
    fn test_link_remote_to_local_skill() {
        let (_tmp, reg) = setup_test_registry();
        reg.create("my-local-skill", "A local skill").unwrap();

        reg.link_remote(
            "my-local-skill",
            "https://github.com/owner/repo",
            Some("skills/my-local-skill"),
            "main",
        )
        .unwrap();

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
        reg.link_remote("linked-skill", "https://github.com/o/r", None, "main")
            .unwrap();

        reg.unlink_remote("linked-skill").unwrap();

        let sources = SourcesConfig::load(&reg.dirs.sources_toml()).unwrap();
        let src = &sources.skills["linked-skill"];
        assert_eq!(src.source_type, SourceType::Local);
        assert!(src.url.is_none());
        assert!(src.git_ref.is_none());
    }
}
