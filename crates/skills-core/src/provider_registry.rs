use crate::config::{HubConfig, HubType};
use crate::provider::SkillProvider;

/// Dispatches skill operations to the correct [`SkillProvider`] based on
/// the user's input or a stored provider type string.
///
/// Providers are tried in registration order. The last registered provider
/// that matches wins, which allows hub providers (registered later) to
/// override the default GitHub provider for specific URL patterns.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn SkillProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Create a registry with only the built-in GitHub provider.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(crate::remote::GitHubProvider));
        reg
    }

    /// Create a registry with GitHub + configured hub providers.
    ///
    /// Hub providers are registered after GitHub so they take priority
    /// for inputs they claim (via `can_handle`).
    pub fn with_hubs(hubs: &[HubConfig]) -> Self {
        let mut reg = Self::with_defaults();
        for hub in hubs {
            if !hub.enabled {
                continue;
            }
            match hub.hub_type {
                HubType::Feed => {
                    reg.register(Box::new(crate::hub_feed::FeedProvider::new(hub.clone())));
                }
                HubType::Api => {
                    // API hub provider not yet implemented — skip silently
                    tracing::debug!(hub = %hub.name, "Skipping API hub (not yet implemented)");
                }
            }
        }
        reg
    }

    /// Register a provider. Later registrations take priority for `detect()`.
    pub fn register(&mut self, provider: Box<dyn SkillProvider>) {
        self.providers.push(provider);
    }

    /// Find the provider that can handle the given user input.
    ///
    /// Iterates in reverse registration order so that hub providers
    /// (registered after defaults) take priority over GitHub for
    /// URLs they claim.
    pub fn detect(&self, input: &str) -> Option<&dyn SkillProvider> {
        self.providers
            .iter()
            .rev()
            .find(|p| p.can_handle(input))
            .map(|p| p.as_ref())
    }

    /// Look up a provider by its type string (e.g., "github", "hub", "feed").
    ///
    /// Used when loading from staging meta.json or sources.toml where the
    /// provider type was previously persisted.
    pub fn by_type(&self, provider_type: &str) -> Option<&dyn SkillProvider> {
        self.providers
            .iter()
            .find(|p| p.provider_type() == provider_type)
            .map(|p| p.as_ref())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_github() {
        let reg = ProviderRegistry::with_defaults();
        assert!(reg.by_type("github").is_some());
    }

    #[test]
    fn test_detect_github_url() {
        let reg = ProviderRegistry::with_defaults();
        let provider = reg.detect("https://github.com/anthropics/skills");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().provider_type(), "github");
    }

    #[test]
    fn test_detect_github_shorthand() {
        let reg = ProviderRegistry::with_defaults();
        let provider = reg.detect("anthropics/skills");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().provider_type(), "github");
    }

    #[test]
    fn test_detect_unknown_returns_none_for_local() {
        let reg = ProviderRegistry::with_defaults();
        // Local paths should not match any provider
        assert!(reg.detect("/usr/local/bin").is_none());
        assert!(reg.detect("./relative/path").is_none());
    }

    #[test]
    fn test_by_type_not_found() {
        let reg = ProviderRegistry::with_defaults();
        assert!(reg.by_type("nonexistent").is_none());
    }
}
