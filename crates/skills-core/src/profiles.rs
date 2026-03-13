use anyhow::{Result, bail};
use std::collections::{BTreeSet, HashSet};

use crate::config::ProfilesConfig;

/// Resolve a profile name into a deduplicated set of skill names,
/// including transitive includes and base skills.
pub fn resolve_profile(
    config: &ProfilesConfig,
    profile_name: &str,
    include_base: bool,
) -> Result<Vec<String>> {
    let mut skills = BTreeSet::new();
    let mut visited = HashSet::new();
    let mut path = Vec::new();

    resolve_recursive(config, profile_name, &mut skills, &mut visited, &mut path)?;

    if include_base {
        for skill in &config.base.skills {
            skills.insert(skill.clone());
        }
    }

    Ok(skills.into_iter().collect())
}

/// Resolve base skills only.
pub fn resolve_base(config: &ProfilesConfig) -> Vec<String> {
    config.base.skills.clone()
}

/// Resolve multiple active profiles + base into a flat deduplicated skill list.
pub fn resolve_active_profiles(
    config: &ProfilesConfig,
    active_profiles: &[String],
) -> Result<Vec<String>> {
    let mut skills = BTreeSet::new();

    // Base skills always included
    for skill in &config.base.skills {
        skills.insert(skill.clone());
    }

    // Each active profile
    for profile_name in active_profiles {
        let profile_skills = resolve_profile(config, profile_name, false)?;
        for skill in profile_skills {
            skills.insert(skill);
        }
    }

    Ok(skills.into_iter().collect())
}

fn resolve_recursive(
    config: &ProfilesConfig,
    profile_name: &str,
    skills: &mut BTreeSet<String>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Result<()> {
    if !visited.insert(profile_name.to_string()) {
        // Circular include detected
        path.push(profile_name.to_string());
        let cycle = path.join(" -> ");
        bail!("Circular include detected: {}", cycle);
    }
    path.push(profile_name.to_string());

    let profile = config
        .profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;

    // Add this profile's own skills
    for skill in &profile.skills {
        skills.insert(skill.clone());
    }

    // Recursively resolve includes
    for included in &profile.includes {
        resolve_recursive(config, included, skills, visited, path)?;
    }

    path.pop();
    Ok(())
}

/// Validate that a profile config has no circular includes.
/// Returns Ok(()) if valid, Err with the cycle description if not.
pub fn validate_no_cycles(config: &ProfilesConfig) -> Result<()> {
    for profile_name in config.profiles.keys() {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        resolve_recursive(
            config,
            profile_name,
            &mut BTreeSet::new(),
            &mut visited,
            &mut path,
        )?;
    }
    Ok(())
}

/// Validate that all skill names referenced in profiles exist in the registry.
pub fn validate_skills_exist(config: &ProfilesConfig, registry_skills: &[String]) -> Vec<String> {
    let known: HashSet<&str> = registry_skills.iter().map(|s| s.as_str()).collect();
    let mut missing = Vec::new();

    for skill in &config.base.skills {
        if !known.contains(skill.as_str()) {
            missing.push(skill.clone());
        }
    }

    for profile in config.profiles.values() {
        for skill in &profile.skills {
            if !known.contains(skill.as_str()) && !missing.contains(skill) {
                missing.push(skill.clone());
            }
        }
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BaseConfig, GlobalConfig, ProfileDef};
    use std::collections::BTreeMap;

    fn make_config() -> ProfilesConfig {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "rust".into(),
            ProfileDef {
                description: Some("Rust".into()),
                skills: vec!["rust-engineer".into(), "cargo-patterns".into()],
                includes: vec![],
            },
        );
        profiles.insert(
            "react".into(),
            ProfileDef {
                description: Some("React".into()),
                skills: vec!["react-specialist".into()],
                includes: vec![],
            },
        );
        profiles.insert(
            "rust-react".into(),
            ProfileDef {
                description: Some("Full-stack".into()),
                skills: vec!["api-design".into()],
                includes: vec!["rust".into(), "react".into()],
            },
        );

        ProfilesConfig {
            global: GlobalConfig::default(),
            base: BaseConfig {
                skills: vec!["code-review".into(), "obsidian".into()],
            },
            profiles,
        }
    }

    #[test]
    fn test_resolve_simple_profile() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust", true).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(skills.contains(&"cargo-patterns".to_string()));
        assert!(skills.contains(&"code-review".to_string())); // base included
    }

    #[test]
    fn test_resolve_without_base() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust", false).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(!skills.contains(&"code-review".to_string())); // base excluded
    }

    #[test]
    fn test_resolve_composite_profile() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust-react", true).unwrap();
        assert!(skills.contains(&"api-design".to_string()));
        assert!(skills.contains(&"rust-engineer".to_string())); // from rust include
        assert!(skills.contains(&"react-specialist".to_string())); // from react include
        assert!(skills.contains(&"code-review".to_string())); // base
    }

    #[test]
    fn test_resolve_deduplicates() {
        let config = make_config();
        let skills = resolve_profile(&config, "rust-react", true).unwrap();
        let unique: HashSet<&String> = skills.iter().collect();
        assert_eq!(skills.len(), unique.len());
    }

    #[test]
    fn test_circular_include_detected() {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "a".into(),
            ProfileDef {
                description: None,
                skills: vec!["s1".into()],
                includes: vec!["b".into()],
            },
        );
        profiles.insert(
            "b".into(),
            ProfileDef {
                description: None,
                skills: vec!["s2".into()],
                includes: vec!["a".into()],
            },
        );
        let config = ProfilesConfig {
            global: GlobalConfig::default(),
            base: BaseConfig::default(),
            profiles,
        };
        let result = resolve_profile(&config, "a", false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular include detected"));
    }

    #[test]
    fn test_nonexistent_profile_fails() {
        let config = make_config();
        assert!(resolve_profile(&config, "nonexistent", false).is_err());
    }

    #[test]
    fn test_resolve_active_profiles() {
        let config = make_config();
        let skills = resolve_active_profiles(&config, &["rust".into(), "react".into()]).unwrap();
        assert!(skills.contains(&"rust-engineer".to_string()));
        assert!(skills.contains(&"react-specialist".to_string()));
        assert!(skills.contains(&"code-review".to_string()));
    }

    #[test]
    fn test_validate_no_cycles_ok() {
        let config = make_config();
        assert!(validate_no_cycles(&config).is_ok());
    }

    #[test]
    fn test_validate_skills_exist() {
        let config = make_config();
        let registry = vec![
            "code-review".into(),
            "obsidian".into(),
            "rust-engineer".into(),
        ];
        let missing = validate_skills_exist(&config, &registry);
        assert!(missing.contains(&"cargo-patterns".to_string()));
        assert!(missing.contains(&"react-specialist".to_string()));
        assert!(!missing.contains(&"rust-engineer".to_string()));
    }

    #[test]
    fn test_transitive_three_levels() {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "c".into(),
            ProfileDef {
                description: None,
                skills: vec!["s3".into()],
                includes: vec![],
            },
        );
        profiles.insert(
            "b".into(),
            ProfileDef {
                description: None,
                skills: vec!["s2".into()],
                includes: vec!["c".into()],
            },
        );
        profiles.insert(
            "a".into(),
            ProfileDef {
                description: None,
                skills: vec!["s1".into()],
                includes: vec!["b".into()],
            },
        );
        let config = ProfilesConfig {
            global: GlobalConfig::default(),
            base: BaseConfig::default(),
            profiles,
        };
        let skills = resolve_profile(&config, "a", false).unwrap();
        assert!(skills.contains(&"s1".to_string()));
        assert!(skills.contains(&"s2".to_string()));
        assert!(skills.contains(&"s3".to_string()));
    }
}
