
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::{Manifest, ProjectConfig};

pub struct ProjectInitializer {
    name: String,
    main_class: Option<String>,
    java_version: Option<String>,
}

impl ProjectInitializer {
    pub fn new(
        name: String,
        main_class: Option<String>,
        java_version: Option<String>,
    ) -> Self {
        Self {
            name,
            main_class,
            java_version,
        }
    }

    pub fn init(&self, path: &Path) -> anyhow::Result<()> {
        // Create project structure
        let src_dir = path.join("src").join("main").join("java");
        let test_dir = path.join("src").join("test").join("java");
        let resource_dir = path.join("src").join("main").join("resources");

        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&test_dir)?;
        fs::create_dir_all(&resource_dir)?;

        let manifest = Manifest {
            project: ProjectConfig {
                name: self.name.clone(),
                version: "0.1.0".to_string(),
                main_class: self.main_class.clone(),
                java_version: self.java_version.clone(),
            },
            deps: Default::default(),
        };

        manifest.save(&path.join("gallade.toml"))?;

        fs::write(
            path.join(".gitignore"),
            ".gallade/\ntarget/\n*.class\n*.jar\n.idea/\n.vscode/\n",
        )?;

        if let Some(main_class) = &self.main_class {
            let main_file = self.create_main_file(main_class, &src_dir)?;
            self.write_hello_world(&main_file)?;
        }

        // Initialize git repo
        if !path.join(".git").exists() {
            Command::new("git")
                .arg("init")
                .current_dir(path)
                .output()?;
        }

        Ok(())
    }

    fn create_main_file(&self, main_class: &str, src_dir: &Path) -> anyhow::Result<PathBuf> {
        let parts: Vec<&str> = main_class.split('.').collect();
        let (package_parts, class_name) = parts.split_at(parts.len() - 1);

        let mut current = src_dir.to_path_buf();
        for part in package_parts {
            current = current.join(part);
        }

        fs::create_dir_all(&current)?;
        Ok(current.join(format!("{}.java", class_name[0])))
    }

    fn write_hello_world(&self, path: &Path) -> anyhow::Result<()> {
        let main_class = self.main_class.as_ref().unwrap();
        let package = main_class.rsplit_once('.').map(|(p, _)| p).unwrap_or("");

        let content = if !package.is_empty() {
            format!(
                "package {};\n\npublic class {} {{\n    public static void main(String[] args) {{\n        System.out.println(\"Hello from Gallade!\");\n    }}\n}}\n",
                package,
                path.file_stem().unwrap().to_str().unwrap()
            )
        } else {
            format!(
                "public class {} {{\n    public static void main(String[] args) {{\n        System.out.println(\"Hello from Gallade!\");\n    }}\n}}\n",
                path.file_stem().unwrap().to_str().unwrap()
            )
        };

        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_init() -> anyhow::Result<()> {
        let temp = TempDir::new()?;

        let initializer = ProjectInitializer::new(
            "test-project".to_string(),
            Some("com.example.Main".to_string()),
            Some("17".to_string()),
        );

        initializer.init(temp.path())?;

        // Check project structure
        assert!(temp.path().join("src/main/java").exists());
        assert!(temp.path().join("src/test/java").exists());
        assert!(temp.path().join("src/main/resources").exists());

        // Check manifest
        assert!(temp.path().join("gallade.toml").exists());

        // Check main class
        assert!(temp.path().join("src/main/java/com/example/Main.java").exists());

        // Check gitignore
        assert!(temp.path().join(".gitignore").exists());
        assert!(temp.path().join(".git").exists());

        Ok(())
    }
}