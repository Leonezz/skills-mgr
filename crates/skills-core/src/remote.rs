use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use std::path::{Path, PathBuf};
use tar::Archive;

/// Parsed information from a GitHub-style URL.
#[derive(Debug, Clone)]
pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: String,
    pub subpath: Option<String>,
}

/// Parse a URL or shorthand into a GitHubSource.
///
/// Supported formats:
///   - `https://github.com/owner/repo`
///   - `https://github.com/owner/repo/tree/ref/path/to/skill`
///   - `owner/repo` (shorthand, defaults to `main`)
///   - `owner/repo/path/to/skill` (shorthand with subpath)
pub fn parse_github_url(input: &str) -> Result<GitHubSource> {
    // Try full URL first
    if input.starts_with("https://github.com/") || input.starts_with("http://github.com/") {
        return parse_github_full_url(input);
    }

    // Shorthand: owner/repo or owner/repo/subpath
    let parts: Vec<&str> = input.splitn(3, '/').collect();
    if parts.len() < 2 {
        bail!("Invalid GitHub reference: '{}'. Expected owner/repo or a GitHub URL.", input);
    }

    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    let subpath = if parts.len() == 3 && !parts[2].is_empty() {
        Some(parts[2].to_string())
    } else {
        None
    };

    Ok(GitHubSource {
        owner,
        repo,
        git_ref: "main".to_string(),
        subpath,
    })
}

fn parse_github_full_url(url: &str) -> Result<GitHubSource> {
    let parsed = url::Url::parse(url).context("Invalid URL")?;
    let segments: Vec<&str> = parsed
        .path_segments()
        .context("URL has no path")?
        .filter(|s| !s.is_empty())
        .collect();

    if segments.len() < 2 {
        bail!("GitHub URL must contain owner/repo: {}", url);
    }

    let owner = segments[0].to_string();
    // Strip .git suffix if present
    let repo = segments[1].trim_end_matches(".git").to_string();

    // https://github.com/owner/repo/tree/ref/path/to/skill
    if segments.len() >= 4 && segments[2] == "tree" {
        let git_ref = segments[3].to_string();
        let subpath = if segments.len() > 4 {
            Some(segments[4..].join("/"))
        } else {
            None
        };
        return Ok(GitHubSource {
            owner,
            repo,
            git_ref,
            subpath,
        });
    }

    Ok(GitHubSource {
        owner,
        repo,
        git_ref: "main".to_string(),
        subpath: None,
    })
}

/// Returns true if the input looks like a remote source (URL or shorthand).
pub fn is_remote_source(input: &str) -> bool {
    input.starts_with("https://") || input.starts_with("http://") || {
        let parts: Vec<&str> = input.splitn(3, '/').collect();
        parts.len() >= 2
            && !input.starts_with('/')
            && !input.starts_with('.')
            && !input.starts_with('~')
            && !Path::new(input).exists()
    }
}

/// Metadata for a skill discovered in a remote repo.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RemoteSkillEntry {
    /// Skill name (directory name)
    pub name: String,
    /// Description from SKILL.md frontmatter, if parseable
    pub description: Option<String>,
    /// Subpath within the repo (e.g. "skills/pdf")
    pub subpath: String,
}

/// Download a GitHub repo tarball and extract to a temp directory.
///
/// Returns (TempDir, top_dir_path). The caller owns the TempDir lifetime.
async fn download_github_tarball(source: &GitHubSource) -> Result<(tempfile::TempDir, PathBuf)> {
    let tarball_url = format!(
        "https://api.github.com/repos/{}/{}/tarball/{}",
        source.owner, source.repo, source.git_ref
    );

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        "skills-mgr/0.1".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );

    // Support GITHUB_TOKEN for private repos and higher rate limits
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse()
                .context("Invalid GITHUB_TOKEN")?,
        );
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&tarball_url)
        .send()
        .await
        .context("Failed to download tarball")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!(
            "GitHub API returned {}: {}",
            status,
            if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body
            }
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    let tmp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(tmp_dir.path())
        .context("Failed to extract tarball")?;

    let top_dir = find_single_child_dir(tmp_dir.path())
        .context("Tarball doesn't contain a single top-level directory")?;

    Ok((tmp_dir, top_dir))
}

/// Download a skill from a GitHub repository to a temporary directory.
///
/// Returns the path to the extracted skill directory inside the temp dir.
/// The caller is responsible for using the contents and cleaning up.
pub async fn download_github_skill(
    source: &GitHubSource,
) -> Result<(tempfile::TempDir, PathBuf)> {
    let (tmp_dir, top_dir) = download_github_tarball(source).await?;

    // Resolve the target path (subpath within the repo)
    let skill_dir = match &source.subpath {
        Some(subpath) => {
            let target = top_dir.join(subpath);
            if !target.exists() {
                bail!(
                    "Subpath '{}' not found in {}/{}@{}",
                    subpath,
                    source.owner,
                    source.repo,
                    source.git_ref
                );
            }
            target
        }
        None => top_dir,
    };

    // Validate that SKILL.md exists
    if !skill_dir.join("SKILL.md").exists() {
        bail!(
            "No SKILL.md found at {}. This doesn't look like a skill directory.",
            skill_dir.display()
        );
    }

    Ok((tmp_dir, skill_dir))
}

/// List all skills available in a remote GitHub repository.
///
/// Downloads the repo tarball and scans for directories containing SKILL.md.
/// Returns a list of discovered skills with their subpaths.
pub async fn list_remote_skills(source: &GitHubSource) -> Result<Vec<RemoteSkillEntry>> {
    let (_tmp_dir, top_dir) = download_github_tarball(source).await?;

    // Resolve base directory if subpath is given
    let base_dir = match &source.subpath {
        Some(subpath) => {
            let target = top_dir.join(subpath);
            if !target.exists() {
                bail!(
                    "Subpath '{}' not found in {}/{}@{}",
                    subpath,
                    source.owner,
                    source.repo,
                    source.git_ref
                );
            }
            target
        }
        None => top_dir.clone(),
    };

    let mut skills = Vec::new();
    scan_for_skills(&base_dir, &top_dir, &mut skills)?;
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Download a GitHub repo to a staging directory and scan for skills.
///
/// Returns the list of discovered skills. The staging directory persists
/// until `clean_staging` is called, enabling `import_from_browse` to
/// import without re-downloading.
pub async fn download_to_staging(
    source: &GitHubSource,
    staging_dir: &Path,
) -> Result<Vec<RemoteSkillEntry>> {
    // Clean any previous staging
    if staging_dir.exists() {
        std::fs::remove_dir_all(staging_dir)?;
    }
    std::fs::create_dir_all(staging_dir)?;

    // Download and extract
    let (tmp_dir, top_dir) = download_github_tarball(source).await?;

    // Copy extracted content to staging/repo/
    let repo_dir = staging_dir.join("repo");
    crate::registry::copy_dir_recursive(&top_dir, &repo_dir)?;
    drop(tmp_dir); // clean up temp

    // Write source metadata
    let meta = serde_json::json!({
        "owner": source.owner,
        "repo": source.repo,
        "git_ref": source.git_ref,
        "subpath": source.subpath,
    });
    std::fs::write(
        staging_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    // Resolve base directory if subpath is given
    let base_dir = match &source.subpath {
        Some(subpath) => {
            let target = repo_dir.join(subpath);
            if !target.exists() {
                bail!(
                    "Subpath '{}' not found in {}/{}@{}",
                    subpath,
                    source.owner,
                    source.repo,
                    source.git_ref
                );
            }
            target
        }
        None => repo_dir.clone(),
    };

    let mut skills = Vec::new();
    scan_for_skills(&base_dir, &repo_dir, &mut skills)?;
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Scan a local directory for skills (used after download).
pub fn scan_directory_for_skills(dir: &Path) -> Result<Vec<RemoteSkillEntry>> {
    let mut skills = Vec::new();
    scan_for_skills(dir, dir, &mut skills)?;
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Recursively scan directories for SKILL.md files, up to 3 levels deep.
fn scan_for_skills(
    dir: &Path,
    repo_root: &Path,
    skills: &mut Vec<RemoteSkillEntry>,
) -> Result<()> {
    scan_for_skills_inner(dir, repo_root, skills, 0)
}

fn scan_for_skills_inner(
    dir: &Path,
    repo_root: &Path,
    skills: &mut Vec<RemoteSkillEntry>,
    depth: u32,
) -> Result<()> {
    if depth > 3 || !dir.is_dir() {
        return Ok(());
    }

    let skill_md = dir.join("SKILL.md");
    if skill_md.exists() {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let subpath = dir
            .strip_prefix(repo_root)
            .unwrap_or(dir)
            .to_string_lossy()
            .to_string();
        let description = parse_skill_description(&skill_md);

        skills.push(RemoteSkillEntry {
            name,
            description,
            subpath,
        });
        // Don't recurse into skill directories — a SKILL.md marks a leaf
        return Ok(());
    }

    // Recurse into subdirectories
    let entries = std::fs::read_dir(dir);
    if let Ok(entries) = entries {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                scan_for_skills_inner(&entry.path(), repo_root, skills, depth + 1)?;
            }
        }
    }

    Ok(())
}

/// Parse description from a SKILL.md file's YAML frontmatter.
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
            let desc = desc.trim().trim_matches('"').trim_matches('\'');
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

/// Find the single child directory inside a directory (GitHub tarball structure).
fn find_single_child_dir(parent: &Path) -> Result<PathBuf> {
    let mut entries = std::fs::read_dir(parent)?;
    let first = entries
        .next()
        .context("Empty directory")?
        .context("Failed to read directory entry")?;

    if first.path().is_dir() {
        Ok(first.path())
    } else {
        bail!("Expected a directory, found a file")
    }
}

/// Build the canonical URL for display and storage in sources.toml.
pub fn canonical_url(source: &GitHubSource) -> String {
    match &source.subpath {
        Some(subpath) => format!(
            "https://github.com/{}/{}/tree/{}/{}",
            source.owner, source.repo, source.git_ref, subpath
        ),
        None => format!(
            "https://github.com/{}/{}",
            source.owner, source.repo
        ),
    }
}

/// Derive a skill name from the source.
/// Uses the last segment of the subpath, or the repo name.
pub fn derive_skill_name(source: &GitHubSource) -> String {
    if let Some(subpath) = &source.subpath {
        subpath
            .rsplit('/')
            .next()
            .unwrap_or(&source.repo)
            .to_string()
    } else {
        source.repo.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_full_url() {
        let s = parse_github_url("https://github.com/anthropics/skills/tree/main/skills/code-review").unwrap();
        assert_eq!(s.owner, "anthropics");
        assert_eq!(s.repo, "skills");
        assert_eq!(s.git_ref, "main");
        assert_eq!(s.subpath.as_deref(), Some("skills/code-review"));
    }

    #[test]
    fn test_parse_github_repo_only() {
        let s = parse_github_url("https://github.com/user/my-skill").unwrap();
        assert_eq!(s.owner, "user");
        assert_eq!(s.repo, "my-skill");
        assert_eq!(s.git_ref, "main");
        assert!(s.subpath.is_none());
    }

    #[test]
    fn test_parse_github_repo_with_git_suffix() {
        let s = parse_github_url("https://github.com/user/my-skill.git").unwrap();
        assert_eq!(s.repo, "my-skill");
    }

    #[test]
    fn test_parse_shorthand_repo_only() {
        let s = parse_github_url("anthropics/skills").unwrap();
        assert_eq!(s.owner, "anthropics");
        assert_eq!(s.repo, "skills");
        assert_eq!(s.git_ref, "main");
        assert!(s.subpath.is_none());
    }

    #[test]
    fn test_parse_shorthand_with_subpath() {
        let s = parse_github_url("anthropics/skills/skills/code-review").unwrap();
        assert_eq!(s.owner, "anthropics");
        assert_eq!(s.repo, "skills");
        assert_eq!(s.subpath.as_deref(), Some("skills/code-review"));
    }

    #[test]
    fn test_derive_skill_name_from_subpath() {
        let s = GitHubSource {
            owner: "anthropics".into(),
            repo: "skills".into(),
            git_ref: "main".into(),
            subpath: Some("skills/code-review".into()),
        };
        assert_eq!(derive_skill_name(&s), "code-review");
    }

    #[test]
    fn test_derive_skill_name_from_repo() {
        let s = GitHubSource {
            owner: "user".into(),
            repo: "my-skill".into(),
            git_ref: "main".into(),
            subpath: None,
        };
        assert_eq!(derive_skill_name(&s), "my-skill");
    }

    #[test]
    fn test_canonical_url_with_subpath() {
        let s = GitHubSource {
            owner: "anthropics".into(),
            repo: "skills".into(),
            git_ref: "main".into(),
            subpath: Some("skills/code-review".into()),
        };
        assert_eq!(
            canonical_url(&s),
            "https://github.com/anthropics/skills/tree/main/skills/code-review"
        );
    }

    #[test]
    fn test_canonical_url_repo_only() {
        let s = GitHubSource {
            owner: "user".into(),
            repo: "my-skill".into(),
            git_ref: "main".into(),
            subpath: None,
        };
        assert_eq!(canonical_url(&s), "https://github.com/user/my-skill");
    }

    #[test]
    fn test_is_remote_source() {
        assert!(is_remote_source("https://github.com/user/repo"));
        assert!(is_remote_source("anthropics/skills"));
        assert!(is_remote_source("user/repo/path/to/skill"));
        assert!(!is_remote_source("/local/path"));
        assert!(!is_remote_source("./relative/path"));
    }

    #[test]
    fn test_scan_for_skills_multi_skill_repo() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        // Simulate a multi-skill repo like anthropics/skills
        // root/skills/pdf/SKILL.md
        // root/skills/docx/SKILL.md
        // root/README.md (no SKILL.md at root)
        std::fs::create_dir_all(root.join("skills/pdf")).unwrap();
        std::fs::write(
            root.join("skills/pdf/SKILL.md"),
            "---\nname: pdf\ndescription: PDF processing\n---\n",
        )
        .unwrap();

        std::fs::create_dir_all(root.join("skills/docx")).unwrap();
        std::fs::write(
            root.join("skills/docx/SKILL.md"),
            "---\nname: docx\ndescription: Word document handling\n---\n",
        )
        .unwrap();

        std::fs::write(root.join("README.md"), "# Skills collection").unwrap();

        let mut skills = Vec::new();
        scan_for_skills(root, root, &mut skills).unwrap();
        skills.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "docx");
        assert_eq!(skills[0].description.as_deref(), Some("Word document handling"));
        assert_eq!(skills[0].subpath, "skills/docx");
        assert_eq!(skills[1].name, "pdf");
        assert_eq!(skills[1].description.as_deref(), Some("PDF processing"));
        assert_eq!(skills[1].subpath, "skills/pdf");
    }

    #[test]
    fn test_scan_for_skills_single_skill_at_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        // Single skill repo: SKILL.md at root
        std::fs::write(
            root.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A single skill\n---\n",
        )
        .unwrap();

        let mut skills = Vec::new();
        scan_for_skills(root, root, &mut skills).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].description.as_deref(), Some("A single skill"));
    }

    #[test]
    fn test_scan_for_skills_empty_repo() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut skills = Vec::new();
        scan_for_skills(tmp.path(), tmp.path(), &mut skills).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_parse_skill_description_valid() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("SKILL.md");
        std::fs::write(&path, "---\nname: test\ndescription: Hello world\n---\nBody").unwrap();
        assert_eq!(parse_skill_description(&path), Some("Hello world".to_string()));
    }

    #[test]
    fn test_parse_skill_description_no_frontmatter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("SKILL.md");
        std::fs::write(&path, "# Just markdown").unwrap();
        assert_eq!(parse_skill_description(&path), None);
    }
}
