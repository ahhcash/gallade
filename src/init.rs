use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;
use rust_embed::RustEmbed;
use tera::{Context, Tera};
use crate::manifest::{Manifest, ProjectConfig};


#[derive(RustEmbed)]
#[folder = "src/templates"]
struct Templates;

pub struct ProjectInitializer {
    group_id: String,
    artifact_id: String,
    version: String,
    java_version: Option<String>,
}

impl ProjectInitializer {
    pub fn new(
        group_id: String,
        artifact_id: String,
        version: String,
        java_version: Option<String>,
    ) -> Self {
        Self {
            group_id,
            artifact_id,
            version,
            java_version,
        }
    }


    pub fn init(&self, path: &Path) -> anyhow::Result<()> {
        let artifact_path = path.join(&self.artifact_id);

        let src_dir = artifact_path.join("src").join("main").join("java");
        let test_dir = artifact_path.join("src").join("test").join("java");
        let resource_dir = artifact_path.join("src").join("main").join("resources");


        fs::create_dir_all(&artifact_path)?;
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&test_dir)?;
        fs::create_dir_all(&resource_dir)?;

        let package_parts: Vec<&str> = self.group_id
            .split('.')
            .collect();

        let package_path = package_parts
            .iter()
            .map(|s| s.to_string())
            .chain(std::iter::once(self.artifact_id.clone()))
            .collect::<Vec<String>>()
            .join("/");

        let package_dir = src_dir.join(&package_path);

        let main_class = format!("{}.{}.App", self.group_id, self.artifact_id);

        let manifest = Manifest {
            project: ProjectConfig {
                name: self.artifact_id.clone(),
                version: "0.1.0".to_string(),
                main_class: Some(main_class.clone()),
                java_version: self.java_version.clone(),
            },
            deps: Default::default(),
        };

        manifest.save(&artifact_path.join("gallade.toml"))?;

        fs::write(
            artifact_path.join(".gitignore"),
            ".gallade/\ntarget/\n*.class\n*.jar\n.idea/\n.vscode/\n",
        )?;


        let app_file = package_dir.join("App.java");
        self.create_app_file(&app_file)?;

        if !artifact_path.join(".git").exists() {
            Command::new("git")
                .arg("init")
                .current_dir(artifact_path)
                .output()?;
        }

        Ok(())
    }

    fn create_app_file(&self, dest: &Path) -> anyhow::Result<()> {
        let template_content = Templates::get("java/App.java")
            .ok_or_else(|| anyhow::anyhow!("App.java template not found"))?;

        let template_str = std::str::from_utf8(&template_content.data)
            .map_err(|e| anyhow::anyhow!("invalid template encoding: {}", e))?;

        let mut tera = Tera::default();
        tera.add_raw_template("app", template_str)?;

        let mut context = Context::new();
        context.insert("package", &self.group_id);

        let content = tera.render("app", &context)?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(dest, content)?;
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tempfile::TempDir;
//     use std::env;
//
//     #[test]
//     fn test_project_init() -> anyhow::Result<()> {
//         let temp = TempDir::new()?;
//
//         let initializer = ProjectInitializer::new(
//             "com.example.test".to_string(),
//             "test".to_string(),
//             "1".to_string(),
//             Some("23".to_string()),
//         );
//         initializer.init(temp.path())?;
//
//         // Check project structure
//         assert!(temp.path().join("src/main/java/com/example/test").exists());
//         assert!(temp.path().join("src/test/java").exists());
//         assert!(temp.path().join("src/main/resources").exists());
//
//         // Check manifest
//         assert!(temp.path().join("gallade.toml").exists());
//
//         // Check main class exists
//         assert!(temp.path().join("src/main/java/com/example/test/App.java").exists());
//
//         // Check gitignore
//         assert!(temp.path().join(".gitignore").exists());
//         assert!(temp.path().join(".git").exists());
//
//         Ok(())
//     }
// }