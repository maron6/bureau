//! Bureau Skills System  
//! ADR3 describes full layout (discovery paths/stacked precedence/frontmatter schema/prompt assembly/error handling)

use anyhow::Result;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

// ============================================================================
// Skill struct — represents one skill loaded from a directory + SKILL.md.
// Frontmatter parsed from SKILL.md defines metadata; markdown body is content.
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    #[serde(default)] pub required_tools: Vec<String>,
   #[serde(default)] pub disabled_tools: Vec<String>,   
    #[serde(default)] pub scope_read: Vec<String>,
}

// ============================================================================
// SkillsLoader — 4 stacked discovery paths per ADR3 / Q8.
// Merge strategy: global-shared → office-shared → global-mode → office-mode
// Last-wins within tiers; first-seen name preserved in output order.
// ============================================================================

/// Load all skills from configured discovery paths (follows ADR3 stacking order). 
pub struct SkillsLoader {
   pub global_shared:PathBuf,
    pub office_shared: PathBuf,
    pub global_mode: PathBuf,     
    pub office_mode: PathBuf,
   pub mode: String,
}

impl SkillsLoader {
    /// Create loader for the given mode targeting this office directory.
    pub fn new(mode: &str, office_path: &Path) -> Self {
        let g = dirs::home_dir().unwrap_or_default().join(".bureau").join("skills");  
       let os = office_path.join(".bureau").join("skills");

        Self {  
            global_shared: g.join("shared"),
            office_shared: os.join("shared"),
            global_mode: g.join(mode), 
            office_mode: os.join(mode),
            mode: mode.to_lowercase(),
        }
    } 

    /// Load all discovered skills from the 4 paths in stacking order.
    /// Returns skills sorted alphabetically by name for deterministic assembly.
    pub fn load_all_skills(&self) -> Result<Vec<Skill>> {
        let mut seen = std::collections::HashMap::new();

        // ADR3 / Q8: iterate paths in precedence order (later overrides earlier).
        for path in &[
            &self.global_shared,
            &self.office_shared,
            &self.global_mode,
            &self.office_mode,
        ] {
            if !path.exists() { continue; }

            let entries = match std::fs::read_dir(path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() { continue; }

                // SKILL.md required per ADR3 — skip directories without it.
                let skill_md = dir.join("SKILL.md");
                if !skill_md.exists() {
                    eprintln!("warning: {} exists but has no SKILL.md — skipping", dir.display());
                    continue;
                }

                match parse_skill_file(&dir) {
                    Ok(skill) => { seen.insert(skill.name.clone(), skill); }   // last-wins per name
                    Err(e) => eprintln!("warning: failed to load skill at {}: {}", dir.display(), e),
                }
            }
        }

        // Sort alphabetically for deterministic prompt assembly order.
        let mut skills = seen.into_values().collect::<Vec<_>>();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    /// Load a single skill by name; returns first-found in stack order (highest precedence).
    pub fn load_skill(&self, name: &str) -> Option<Skill> {
        self.load_all_skills()
            .ok()
            .and_then(|s| s.into_iter().find(|sk| sk.name == name))
    }

    /// Assemble the full prompt context by concatenating discovered skill contents.
    pub fn assembly_prompt(&self) -> Result<String> {
        let skills = self.load_all_skills()?;
        if skills.is_empty() { return Ok(String::new()); }

        let mut parts: Vec<String> = vec![
            "## BUREAU PROMPT ASSEMBLY\n".into(),
            format!("## Mode: {}\n\n", self.mode),
        ];

        for skill in &skills {
            parts.push(format!(
                "<skill name=\"{}\" desc=\"{}\">\n{}\n</skill>\n\n",
                skill.name, skill.description, skill.content
            ));
        }

        Ok(parts.concat())
    }

    /// List all discovered skill names (for TUI command palette suggestions).
    pub fn list_skill_names(&self) -> Result<Vec<String>> {
        self.load_all_skills().map(|s| s.into_iter().map(|sk| sk.name).collect())
    }
}

// ============================================================================
// parse_skill_file — reads a SKILL.md file and returns a Skill.
// Skips YAML frontmatter (if present); remaining content is the free-form body.
// Per ADR3/Q11: frontmatter schema = name, description, scope_read, disabled_tools.
// ============================================================================

/// Parse one skill directory's SKILL.md into a Skill struct.
fn parse_skill_file(dir: &Path) -> Result<Skill> {
    let md_path = dir.join("SKILL.md");
    let content = std::fs::read_to_string(&md_path)?;

    let mut name = String::new();
    let mut desc = String::new();
    let mut scope_read: Vec<String> = vec![];
    let mut disabled_tools: Vec<String> = vec![];
    let required_tools: Vec<String> = vec![]; // TODO: parse from frontmatter (Q11)

    // Extract YAML frontmatter if present.
    let body = if content.starts_with("---") {
        let (_, rest) = content.split_once("---").ok_or("Missing frontmatter closing ---")?;
        if let Some((front, body_part)) = rest.split_once("---") {
            parse_frontmatter(front)?;
            // Parse frontmatter fields from the `front` string.
            for line in front.lines() {
                if let Some(key_val) = line.strip_prefix("name:") {
                    name = key_val.trim().to_string();
                } else if let Some(key_val) = line.strip_prefix("description:") {
                    desc = key_val.trim().to_string();
                } else if let Some(key_val) = line.strip_prefix("scope_read:") {
                    scope_read.push(key_val.trim().to_string());
                } else if let Some(key_val) = line.strip_prefix("disabled_tools:") {
                    disabled_tools.push(key_val.trim().to_string());
                }
            }
            body_part.to_string()
        } else {
            // No closing --- found — treat entire file as markdown content.
            eprintln!("warning: SKILL.md at {} has opening --- but no closing", dir.display());
            content.clone()
        }
    } else {
        // No frontmatter — entire file is free-form Markdown body (Q11 permits this).
        if dir.file_name().map_or(false, |n| n == "name") {
            name = dir.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
        } else {
            // Fall back to directory name as default (ADR3/Q15 grilling decision).
            name = dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".into());
        }
        content
    };

    Ok(Skill {
        name,
        description: desc,
        content: body.trim().to_string(),
        required_tools,
        disabled_tools,
        scope_read,
    })
}

/// Parse frontmatter lines into a simple lookup — used to validate syntax only.
fn parse_frontmatter(frontmatter: &str) -> Result<()> {
    let _: serde_yaml::Value = serde_yaml::from_str(frontmatter).map_err(|e| anyhow!("Invalid YAML in SKILL.md frontmatter: {}", e))?;
    Ok(())
}

// ==========================================================================
// P9: Unit tests for skills loading and stacking precedence
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    #[test]
    fn test_skill_name_fallback_to_directory_name() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "# My skill without frontmatter").unwrap();
        
        let skill = parse_skill_file(&skill_dir).unwrap();
        assert_eq!(skill.name, "my-skill");
    }

    #[test]
    fn test_skill_name_from_frontmatter() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("actual-name");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "name: overriden-name").unwrap();
        writeln!(f, "description: test desc").unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "# Content").unwrap();

        let skill = parse_skill_file(&skill_dir).unwrap();
        assert_eq!(skill.name, "overriden-name");
    }

    #[test]
    fn test_invalid_yaml_fails() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("bad");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "name: broken").unwrap();
        writeln!(f, "[invalid yaml{{{").unwrap(); // invalid YAML
        writeln!(f, "---").unwrap();

        let result = parse_skill_file(&skill_dir);
        assert!(result.is_err());
    }
}
