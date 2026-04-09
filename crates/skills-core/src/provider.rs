use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::Result;

use crate::config::SkillSource;
use crate::remote::RemoteSkillEntry;

/// Boxed future returning a downloaded skill directory.
type DownloadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(tempfile::TempDir, PathBuf)>> + Send + 'a>>;

/// Boxed future returning an optional downloaded skill directory (None = provider declined).
type UpdateFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<(tempfile::TempDir, PathBuf)>>> + Send + 'a>>;

/// Metadata written to staging/meta.json during the browse flow.
///
/// Provider-agnostic: each provider stores what it needs in `provider_data`.
/// The `provider_type` field is used to look up the correct provider when
/// importing from a cached staging directory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagingMeta {
    /// Provider type identifier (e.g., "github", "hub", "feed").
    pub provider_type: String,
    /// The original user input that triggered the browse (URL, shorthand, etc.).
    pub source_input: String,
    /// Provider-specific opaque data needed for import (e.g., owner/repo/ref for GitHub).
    pub provider_data: serde_json::Value,
}

/// A skill source provider that can discover, download, and track skills from
/// a particular kind of remote source (GitHub, a skill hub, a featured feed, etc.).
///
/// Async methods return boxed futures to allow dynamic dispatch via `dyn SkillProvider`.
pub trait SkillProvider: Send + Sync {
    /// Unique type identifier (e.g., "github", "hub", "feed").
    /// Must be stable — it's persisted in staging meta and sources.toml.
    fn provider_type(&self) -> &str;

    /// The hub name for hub-backed providers (e.g., "clawhub").
    /// Defaults to `provider_type()`. Override for hub providers
    /// where the hub name differs from the provider type.
    fn hub_name(&self) -> &str {
        self.provider_type()
    }

    /// Returns `true` if this provider can handle the given user input
    /// (URL, shorthand, hub identifier, etc.).
    fn can_handle(&self, input: &str) -> bool;

    /// Derive a default skill name from the user input.
    fn derive_name(&self, input: &str) -> String;

    /// Download to a staging directory for multi-skill browsing.
    ///
    /// The provider must:
    /// 1. Download/fetch skill content or metadata to `staging_dir`
    /// 2. Write `staging_dir/meta.json` with a [`StagingMeta`]
    /// 3. Return the list of discovered skills
    fn download_to_staging<'a>(
        &'a self,
        input: &'a str,
        staging_dir: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RemoteSkillEntry>>> + Send + 'a>>;

    /// Download a single skill to a temporary directory.
    ///
    /// Returns `(TempDir, path_to_skill_content)`. The caller owns the TempDir lifetime.
    fn download_skill<'a>(&'a self, input: &'a str) -> DownloadFuture<'a>;

    /// Build a [`StagingMeta`] for a direct import (non-browse flow).
    ///
    /// Each provider populates `provider_data` with whatever it needs for
    /// `build_skill_source` to produce correct source metadata.
    /// Default implementation includes just `hub_name`.
    fn build_import_meta(&self, input: &str) -> Result<StagingMeta> {
        Ok(StagingMeta {
            provider_type: self.provider_type().to_string(),
            source_input: input.to_string(),
            provider_data: serde_json::json!({
                "hub_name": self.hub_name(),
            }),
        })
    }

    /// Build a [`SkillSource`] for tracking in sources.toml.
    ///
    /// Called after a skill has been copied into the registry, with:
    /// - `meta`: the staging metadata (from meta.json)
    /// - `subpath`: the skill's subpath within the staged content
    /// - `skill_name`: the resolved skill name
    /// - `hash`: the computed content hash (sha256:...)
    fn build_skill_source(
        &self,
        meta: &StagingMeta,
        subpath: &str,
        skill_name: &str,
        hash: &str,
    ) -> SkillSource;

    /// Re-download a skill for update, given its existing [`SkillSource`].
    ///
    /// Returns `Ok(Some((TempDir, path)))` if the provider can handle this source,
    /// or `Ok(None)` if it cannot (wrong provider type).
    fn download_for_update<'a>(&'a self, source: &'a SkillSource) -> UpdateFuture<'a>;
}
