use std::env;
use std::path::{Path, PathBuf};
use crate::coordinates::Coordinate;
use crate::repository::{ArtifactKind, Repository};


pub type Classpath = String;

#[derive(Debug, Clone)]
pub struct ClasspathBuilder {
    repository: Repository,
    paths: Vec<PathBuf>
}


pub fn resolve_main_class(target_dir: &Path, manifest_main_class: Option<String>) -> anyhow::Result<String> {
    if let Some(main_class) = manifest_main_class {
        let class_path = main_class.replace('.', "/") + ".class";
        let full_path = target_dir.join(&class_path);

        if !full_path.exists() {
            anyhow::bail!(
                "Main class '{}' specified in manifest not found at '{}'",
                main_class,
                full_path.display()
            );
        }
        return Ok(main_class);
    }

    anyhow::bail!("No main class specified in manifest")
}

impl ClasspathBuilder {
    pub fn new(repository: Repository) -> Self {
        ClasspathBuilder {
            repository,
            paths: Vec::new()
        }
    }

    pub fn with_dep(&mut self, coord: &Coordinate, version: &str) -> &Self {
        let jar_path = self.repository.get_artifact_path(coord, version, ArtifactKind::Binary);
        if !jar_path.exists() {
            println!("{} is not a valid path, skipping", jar_path.to_string_lossy());
        }
        self.paths.push(jar_path);
        self
    }

    pub fn with_dir(&mut self, dir: PathBuf) -> &Self {
        self.paths.push(dir);
        self
    }

    pub fn with_repo(&mut self, repo: Repository) -> &Self {
        self.repository = repo;
        self
    }

    pub fn build(&self) -> String {
        let sep = if cfg!(windows) { ";"}  else { ":" };
        self.paths
            .iter()
            .map(|e| e.to_string_lossy().into_owned())
            .collect::<Vec<(String)>>()
            .join(sep)
    }
}

pub struct JavaEnvironment {
    java_home: PathBuf,
    pub classpath: String
}

impl JavaEnvironment {
    pub fn new(classpath: String) -> anyhow::Result<Self> {
        let mut java_home = PathBuf::new();
        if let Ok(path) = env::var("JAVA_HOME") {
            java_home = PathBuf::from(path);
        } else {
            if let Ok(output) = std::process::Command::new("java")
                .arg("-XshowSettings:properties")
                .arg("-version")
                .output() {
                let output = String::from_utf8_lossy(&output.stderr);
                for line in output.lines() {
                    if line.contains("java.home") {
                        if let Some(home) = line.split('=').nth(1) {
                            java_home = PathBuf::from(home.trim());
                        } else {
                            anyhow::bail!("couldn't parse java.home from java -version output");
                        }
                    }
                }
            }
            anyhow::bail!("JAVA_HOME not set")
        };

        if !java_home.exists() {
            anyhow::bail!("JAVA_HOME does not exist: {}", java_home.display())
        }

        Ok(Self {
            java_home,
            classpath
        })
    }
}

