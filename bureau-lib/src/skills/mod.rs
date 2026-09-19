//! Bureau Skills System  
//! ADR3 describes full layout (discovery paths/stacked precedence/frontmatter schema/prompt assembly/error handling)

use anyhow::Result;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    #[serde(default)] pub required_tools: Vec<String>,
   #[serde(default)] pub disabled_tools: Vec<String>,   
    #[serde(default)] pub scope_read: Vec<String>,
}

/// Load all skills from configured paths (follows ADR 3 stacking order). 
pub struct SkillsLoader {
   pub global_shared:PathBuf,
    pub office_shared: PathBuf,
    pub global_mode: PathBuf,     
    pub office_mode: PathBuf,
   pub mode: String,
}

impl SkillsLoader {
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

} 