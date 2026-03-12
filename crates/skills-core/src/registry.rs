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
            },
        );
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
            },
        );
        sources.save(&self.dirs.sources_toml())?;

        Ok(skill_name)
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
}
