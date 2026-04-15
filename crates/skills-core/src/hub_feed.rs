use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{Context, Result, bail};

use crate::config::{HubConfig, SkillSource, SourceType};
use crate::provider::{SkillProvider, StagingMeta};
use crate::remote::{self, RemoteSkillEntry};

/// A skill in a featured feed JSON file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeaturedSkill {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub stars: Option<u64>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub source_url: String,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Top-level structure of a featured skills JSON feed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeaturedFeed {
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    pub skills: Vec<FeaturedSkill>,
}

/// Skill provider for JSON-based featured feeds.
///
/// Fetches a JSON file listing skills with metadata and `source_url` fields.
/// The `source_url` typically points to a GitHub repo, which is then used
/// for the actual content download via the existing GitHub machinery.
pub struct FeedProvider {
    hub: HubConfig,
    client: reqwest::Client,
}

impl FeedProvider {
    pub fn new(hub: HubConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("skills-mgr/0.1")
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        Self { hub, client }
    }

    /// Fetch and parse the feed from the configured URL.
    async fn fetch_feed(&self) -> Result<FeaturedFeed> {
        let response = self
            .client
            .get(&self.hub.base_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch feed from {}", self.hub.base_url))?;

        if !response.status().is_success() {
            bail!(
                "Feed {} returned HTTP {}",
                self.hub.base_url,
                response.status()
            );
        }

        let feed: FeaturedFeed = response
            .json()
            .await
            .with_context(|| format!("Failed to parse feed JSON from {}", self.hub.base_url))?;

        Ok(feed)
    }

    /// Find a skill in the feed by slug.
    async fn find_skill(&self, slug: &str) -> Result<FeaturedSkill> {
        let feed = self.fetch_feed().await?;
        feed.skills
            .into_iter()
            .find(|s| s.slug == slug)
            .with_context(|| {
                format!(
                    "Skill '{}' not found in hub '{}'",
                    slug, self.hub.display_name
                )
            })
    }

    /// Extract a skill slug from a hub page URL.
    ///
    /// For `https://clawhub.ai/pskoett/self-improving-agent`, returns `self-improving-agent`.
    /// Takes the last non-empty path segment.
    fn extract_slug_from_url(&self, url: &str) -> Option<String> {
        let page_url = self.hub.page_url.as_deref()?;
        let path = url.strip_prefix(page_url)?;
        // Strip query params and fragments before extracting slug
        let path = path.split('?').next().unwrap_or(path);
        let path = path.split('#').next().unwrap_or(path);
        // path is e.g. "/pskoett/self-improving-agent" or "/pskoett/self-improving-agent/"
        path.trim_matches('/')
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Check if the input is a hub page URL (e.g., `https://clawhub.ai/owner/skill`).
    fn is_page_url(&self, input: &str) -> bool {
        if let Some(page_url) = &self.hub.page_url
            && input.starts_with(page_url)
        {
            // Must have at least one path segment (not just the base URL)
            let rest = input[page_url.len()..].trim_start_matches('/');
            return !rest.is_empty();
        }
        false
    }

    /// Validate that a slug contains only safe characters (alphanumeric, hyphens, underscores).
    fn validate_slug(slug: &str) -> Result<()> {
        if slug.is_empty() {
            bail!("Skill slug is empty");
        }
        if !slug
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            bail!(
                "Invalid slug '{}': must contain only alphanumeric characters, hyphens, or underscores",
                slug
            );
        }
        Ok(())
    }

    /// Download a skill ZIP from the hub's download API and extract to a temp dir.
    async fn download_zip(&self, slug: &str) -> Result<(tempfile::TempDir, PathBuf)> {
        Self::validate_slug(slug)?;

        let api_url = self
            .hub
            .download_api_url
            .as_deref()
            .with_context(|| format!("Hub '{}' has no download_api_url configured", self.hub.name))?
            .replace("{slug}", slug);

        tracing::info!(url = %api_url, slug = %slug, "Downloading skill ZIP from hub");

        let response = self
            .client
            .get(&api_url)
            .send()
            .await
            .with_context(|| format!("Failed to download from {}", api_url))?;

        if !response.status().is_success() {
            bail!(
                "Hub download returned HTTP {} for slug '{}'",
                response.status(),
                slug
            );
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read ZIP response body")?;

        let tmp_dir = tempfile::TempDir::new().context("Failed to create temp dir")?;
        let extract_dir = tmp_dir.path().join(slug);
        std::fs::create_dir_all(&extract_dir)?;

        // Extract ZIP
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).context("Failed to open ZIP archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            // Skip directories and hidden files
            if file.is_dir() || name.starts_with("__MACOSX") || name.starts_with('.') {
                continue;
            }

            // Guard against path traversal (e.g., "../../etc/malicious" in ZIP)
            let entry_path = std::path::Path::new(&name);
            if entry_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                tracing::warn!(
                    path = %name,
                    "Skipping ZIP entry with path traversal"
                );
                continue;
            }

            let out_path = extract_dir.join(&name);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // If all files ended up in a subdirectory, point to the SKILL.md location
        let skill_md = extract_dir.join("SKILL.md");
        if skill_md.exists() {
            return Ok((tmp_dir, extract_dir));
        }

        // Check one level down (ZIP might have a top-level directory)
        for entry in std::fs::read_dir(&extract_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("SKILL.md").exists() {
                return Ok((tmp_dir, entry.path()));
            }
        }

        // Fall back to the extract dir even without SKILL.md
        Ok((tmp_dir, extract_dir))
    }
}

impl SkillProvider for FeedProvider {
    fn provider_type(&self) -> &str {
        "feed"
    }

    fn hub_name(&self) -> &str {
        &self.hub.name
    }

    fn can_handle(&self, input: &str) -> bool {
        // Handle inputs in the form "hub:hubname/slug" or "hub:hubname"
        if let Some(rest) = input.strip_prefix("hub:") {
            return rest.starts_with(&self.hub.name);
        }
        // Handle direct hub page URLs (e.g., https://clawhub.ai/owner/skill)
        if self.is_page_url(input) {
            return true;
        }
        // Also handle the feed URL directly
        input == self.hub.base_url
    }

    fn derive_name(&self, input: &str) -> String {
        // Extract slug from hub page URL
        if let Some(slug) = self.extract_slug_from_url(input) {
            return slug;
        }
        // Extract slug from "hub:hubname/slug"
        if let Some(rest) = input.strip_prefix("hub:")
            && let Some(slug) = rest
                .strip_prefix(&self.hub.name)
                .and_then(|s| s.strip_prefix('/'))
        {
            return slug.to_string();
        }
        input.to_string()
    }

    fn download_to_staging<'a>(
        &'a self,
        input: &'a str,
        staging_dir: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RemoteSkillEntry>>> + Send + 'a>> {
        Box::pin(async move {
            let feed = self.fetch_feed().await?;

            // Clean previous staging
            if staging_dir.exists() {
                std::fs::remove_dir_all(staging_dir)?;
            }
            std::fs::create_dir_all(staging_dir)?;

            // Convert feed skills to RemoteSkillEntry
            let skills: Vec<RemoteSkillEntry> = feed
                .skills
                .iter()
                .filter(|s| !s.source_url.is_empty())
                .map(|s| RemoteSkillEntry {
                    name: s.name.clone(),
                    description: s.summary.clone(),
                    subpath: s.slug.clone(),
                })
                .collect();

            // Write feed data to staging for later use by import_from_browse
            let feed_cache_path = staging_dir.join("feed.json");
            std::fs::write(
                &feed_cache_path,
                serde_json::to_string_pretty(&feed.skills)?,
            )?;

            // Write provider-agnostic staging meta
            let meta = StagingMeta {
                provider_type: "feed".to_string(),
                source_input: input.to_string(),
                provider_data: serde_json::json!({
                    "hub_name": self.hub.name,
                    "display_name": self.hub.display_name,
                    "feed_url": self.hub.base_url,
                }),
            };
            std::fs::write(
                staging_dir.join("meta.json"),
                serde_json::to_string_pretty(&meta)?,
            )?;

            Ok(skills)
        })
    }

    fn download_skill<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(tempfile::TempDir, PathBuf)>> + Send + 'a>> {
        Box::pin(async move {
            // Extract slug from hub page URL or "hub:hubname/slug"
            let slug = self
                .extract_slug_from_url(input)
                .or_else(|| {
                    input
                        .strip_prefix("hub:")
                        .and_then(|rest| rest.strip_prefix(&self.hub.name))
                        .and_then(|rest| rest.strip_prefix('/'))
                        .map(|s| s.to_string())
                })
                .with_context(|| {
                    format!(
                        "Cannot extract skill slug from '{}' for hub '{}'",
                        input, self.hub.name
                    )
                })?;

            // If the hub has a download API, use it (ZIP download)
            if self.hub.download_api_url.is_some() {
                return self.download_zip(&slug).await;
            }

            // Otherwise fall back to feed lookup → GitHub source_url
            let skill = self.find_skill(&slug).await?;
            let github_source = remote::parse_github_url(&skill.source_url)?;
            remote::download_github_skill(&github_source).await
        })
    }

    fn build_skill_source(
        &self,
        meta: &StagingMeta,
        subpath: &str,
        _skill_name: &str,
        hash: &str,
    ) -> SkillSource {
        let hub_name = meta.provider_data["hub_name"]
            .as_str()
            .unwrap_or(&self.hub.name);

        // Use page_url for user-facing URL if available, else fall back to feed URL
        let url = self
            .hub
            .page_url
            .as_deref()
            .unwrap_or(&self.hub.base_url)
            .to_string();

        SkillSource {
            source_type: SourceType::Hub,
            url: Some(url),
            path: None,
            git_ref: None,
            hash: Some(hash.to_string()),
            updated_at: Some(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string(),
            ),
            original_agent_path: None,
            provider: Some("feed".to_string()),
            hub_name: Some(hub_name.to_string()),
            hub_skill_id: Some(subpath.to_string()),
        }
    }

    fn download_for_update<'a>(
        &'a self,
        source: &'a SkillSource,
    ) -> Pin<Box<dyn Future<Output = Result<Option<(tempfile::TempDir, PathBuf)>>> + Send + 'a>>
    {
        Box::pin(async move {
            // Only handle hub-sourced skills from this hub
            if source.source_type != SourceType::Hub {
                return Ok(None);
            }
            let hub_name = match &source.hub_name {
                Some(n) => n,
                None => return Ok(None),
            };
            // Match against the hub name or the provider type (for backwards compat
            // with entries that stored "feed" instead of the hub name like "clawhub")
            if hub_name != &self.hub.name && hub_name != self.provider_type() {
                return Ok(None);
            }

            let slug = match &source.hub_skill_id {
                Some(id) => id.clone(),
                None => return Ok(None),
            };

            // If we have a download API, use ZIP download directly
            if self.hub.download_api_url.is_some() {
                let result = self.download_zip(&slug).await?;
                return Ok(Some(result));
            }

            // Otherwise re-fetch from feed to get current source_url
            let skill = self.find_skill(&slug).await?;
            let github_source = remote::parse_github_url(&skill.source_url)?;
            let result = remote::download_github_skill(&github_source).await?;
            Ok(Some(result))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HubType;

    fn test_hub() -> HubConfig {
        HubConfig {
            name: "test-feed".to_string(),
            display_name: "Test Feed".to_string(),
            hub_type: HubType::Feed,
            base_url: "https://example.com/skills.json".to_string(),
            enabled: true,
            api_key_env: None,
            page_url: Some("https://test-feed.example.com".to_string()),
            download_api_url: Some(
                "https://test-feed.example.com/api/download?slug={slug}".to_string(),
            ),
        }
    }

    #[test]
    fn test_can_handle() {
        let provider = FeedProvider::new(test_hub());
        assert!(provider.can_handle("hub:test-feed"));
        assert!(provider.can_handle("hub:test-feed/some-skill"));
        assert!(provider.can_handle("https://example.com/skills.json"));
        // Hub page URLs
        assert!(provider.can_handle("https://test-feed.example.com/owner/my-skill"));
        assert!(provider.can_handle("https://test-feed.example.com/my-skill"));
        // Base page URL alone (no path) should NOT match
        assert!(!provider.can_handle("https://test-feed.example.com"));
        assert!(!provider.can_handle("https://test-feed.example.com/"));
        assert!(!provider.can_handle("https://github.com/user/repo"));
        assert!(!provider.can_handle("hub:other-hub/skill"));
    }

    #[test]
    fn test_derive_name() {
        let provider = FeedProvider::new(test_hub());
        assert_eq!(provider.derive_name("hub:test-feed/my-skill"), "my-skill");
        assert_eq!(provider.derive_name("hub:test-feed"), "hub:test-feed");
        // From page URL
        assert_eq!(
            provider.derive_name("https://test-feed.example.com/owner/my-skill"),
            "my-skill"
        );
        assert_eq!(
            provider.derive_name("https://test-feed.example.com/my-skill"),
            "my-skill"
        );
    }

    #[test]
    fn test_extract_slug_from_url() {
        let provider = FeedProvider::new(test_hub());
        assert_eq!(
            provider.extract_slug_from_url("https://test-feed.example.com/owner/my-skill"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            provider.extract_slug_from_url("https://test-feed.example.com/my-skill"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            provider.extract_slug_from_url("https://test-feed.example.com/owner/my-skill/"),
            Some("my-skill".to_string())
        );
        // Query params and fragments are stripped
        assert_eq!(
            provider
                .extract_slug_from_url("https://test-feed.example.com/owner/my-skill?ref=latest"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            provider.extract_slug_from_url("https://test-feed.example.com/owner/my-skill#readme"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            provider.extract_slug_from_url("https://other-site.com/owner/skill"),
            None
        );
    }

    #[test]
    fn test_provider_type() {
        let provider = FeedProvider::new(test_hub());
        assert_eq!(provider.provider_type(), "feed");
    }

    #[test]
    fn test_build_skill_source() {
        let provider = FeedProvider::new(test_hub());
        let meta = StagingMeta {
            provider_type: "feed".to_string(),
            source_input: "hub:test-feed".to_string(),
            provider_data: serde_json::json!({
                "hub_name": "test-feed",
                "display_name": "Test Feed",
                "feed_url": "https://example.com/skills.json",
            }),
        };

        let source = provider.build_skill_source(&meta, "my-skill", "my-skill", "sha256:abc123");

        assert_eq!(source.source_type, SourceType::Hub);
        assert_eq!(source.provider.as_deref(), Some("feed"));
        assert_eq!(source.hub_name.as_deref(), Some("test-feed"));
        assert_eq!(source.hub_skill_id.as_deref(), Some("my-skill"));
        assert_eq!(source.hash.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn test_parse_featured_feed() {
        let json = r#"{
            "updated_at": "2026-04-01T00:00:00Z",
            "total": 2,
            "categories": ["development"],
            "skills": [
                {
                    "slug": "code-review",
                    "name": "Code Review",
                    "summary": "Reviews pull requests",
                    "downloads": 100,
                    "stars": 5,
                    "category": "development",
                    "tags": ["review"],
                    "source_url": "https://github.com/anthropics/skills/tree/main/skills/code-review",
                    "updated_at": "2026-03-15T00:00:00Z"
                },
                {
                    "slug": "pdf",
                    "name": "PDF Processing",
                    "summary": "Handles PDF files",
                    "downloads": 50,
                    "stars": 3,
                    "category": "ai-assistant",
                    "tags": ["pdf"],
                    "source_url": "https://github.com/anthropics/skills/tree/main/skills/pdf",
                    "updated_at": "2026-03-14T00:00:00Z"
                }
            ]
        }"#;

        let feed: FeaturedFeed = serde_json::from_str(json).unwrap();
        assert_eq!(feed.skills.len(), 2);
        assert_eq!(feed.skills[0].slug, "code-review");
        assert_eq!(feed.skills[0].name, "Code Review");
        assert_eq!(feed.skills[0].downloads, Some(100));
        assert_eq!(feed.skills[1].slug, "pdf");
        assert!(feed.skills[1].source_url.contains("github.com"));
    }

    #[test]
    fn test_validate_slug() {
        assert!(FeedProvider::validate_slug("code-review").is_ok());
        assert!(FeedProvider::validate_slug("my_skill_123").is_ok());
        assert!(FeedProvider::validate_slug("").is_err());
        assert!(FeedProvider::validate_slug("../etc/passwd").is_err());
        assert!(FeedProvider::validate_slug("skill?ref=latest").is_err());
        assert!(FeedProvider::validate_slug("skill&foo=bar").is_err());
    }
}
