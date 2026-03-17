use std::path::Path;

/// Extract the YAML frontmatter block from a SKILL.md file.
/// Returns the text between the opening and closing `---` markers.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }
    let end = content[3..].find("---")?;
    Some(&content[3..3 + end])
}

/// Parse a single field from YAML frontmatter.
/// Handles inline values, quoted strings, and multiline scalars (>, |, >-, |+, etc.).
fn parse_field(frontmatter: &str, field: &str) -> Option<String> {
    let prefix = format!("{}:", field);
    let lines: Vec<&str> = frontmatter.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix(&prefix) {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if value.starts_with('>') || value.starts_with('|') {
                // YAML multiline scalar — collect indented continuation lines
                let mut parts = Vec::new();
                for cont in &lines[i + 1..] {
                    if cont.starts_with(' ') || cont.starts_with('\t') {
                        parts.push(cont.trim());
                    } else {
                        break;
                    }
                }
                let sep = if value.starts_with('|') { "\n" } else { " " };
                let joined = parts.join(sep);
                return if joined.is_empty() {
                    None
                } else {
                    Some(joined)
                };
            }
            return if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
    }
    None
}

/// Parse the `name` field from a SKILL.md file's frontmatter.
pub fn parse_name(skill_md: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_md).ok()?;
    let fm = extract_frontmatter(&content)?;
    parse_field(fm, "name")
}

/// Parse the `description` field from a SKILL.md file's frontmatter.
/// Handles inline values and YAML multiline scalars (>, |, and chomp variants).
pub fn parse_description(skill_md: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_md).ok()?;
    let fm = extract_frontmatter(&content)?;
    parse_field(fm, "description")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_inline_description() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: Inline desc\n---\n",
        )
        .unwrap();
        assert_eq!(parse_description(&skill_md), Some("Inline desc".into()));
    }

    #[test]
    fn test_parse_folded_description() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: >\n  Line one\n  line two.\n---\n",
        )
        .unwrap();
        assert_eq!(
            parse_description(&skill_md),
            Some("Line one line two.".into())
        );
    }

    #[test]
    fn test_parse_literal_description() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: |\n  Line one\n  line two\n---\n",
        )
        .unwrap();
        assert_eq!(
            parse_description(&skill_md),
            Some("Line one\nline two".into())
        );
    }

    #[test]
    fn test_parse_chomp_indicator() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(
            &skill_md,
            "---\nname: test\ndescription: >-\n  Stripped trailing newline\n---\n",
        )
        .unwrap();
        assert_eq!(
            parse_description(&skill_md),
            Some("Stripped trailing newline".into())
        );
    }

    #[test]
    fn test_parse_name() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(&skill_md, "---\nname: my-skill\ndescription: Foo\n---\n").unwrap();
        assert_eq!(parse_name(&skill_md), Some("my-skill".into()));
    }

    #[test]
    fn test_no_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let skill_md = tmp.path().join("SKILL.md");
        std::fs::write(&skill_md, "No frontmatter here").unwrap();
        assert_eq!(parse_description(&skill_md), None);
        assert_eq!(parse_name(&skill_md), None);
    }
}
