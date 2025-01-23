use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub main_class: Option<String>,
    pub java_version: Option<String>
}

#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub project: ProjectConfig,
    #[serde(default)]
    pub deps: Dependencies,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DepSpec {
    Simple(String),
    Detailed {
        version: String,
        scope: Option<String>,
        optional: bool
    }
}

pub type Dependencies = HashMap<String, DepSpec>;

impl Manifest {
    pub fn load(path: &Path) -> anyhow::Result<Manifest> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_manifest() -> anyhow::Result<()> {
        let content = r#"
            [project]
            name = "test-project"
            version = "0.1.0"

            [deps]
            "com.google.guava" = "31.1.3"
        "#;

        let temp = NamedTempFile::new()?;
        fs::write(temp.path(), content)?;

        let manifest = Manifest::load(temp.path())?;
        assert_eq!(manifest.project.name, "test-project");

        // Check simple dependency
        match manifest.deps.get("com.google.guava") {
            Some(DepSpec::Simple(version)) => assert_eq!(version, "31.1.3"),
            _ => panic!("expected simple dependency spec"),
        }

        Ok(())
    }
}

