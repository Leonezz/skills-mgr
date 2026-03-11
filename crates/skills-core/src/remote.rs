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

/// Download a skill from a GitHub repository to a temporary directory.
///
/// Returns the path to the extracted skill directory inside the temp dir.
/// The caller is responsible for using the contents and cleaning up.
pub async fn download_github_skill(
    source: &GitHubSource,
) -> Result<(tempfile::TempDir, PathBuf)> {
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

    // Extract tarball
    let tmp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(tmp_dir.path())
        .context("Failed to extract tarball")?;

    // GitHub tarballs have a single top-level directory: owner-repo-sha/
    let top_dir = find_single_child_dir(tmp_dir.path())
        .context("Tarball doesn't contain a single top-level directory")?;

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
}
