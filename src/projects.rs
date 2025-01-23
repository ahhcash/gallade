use std::fmt::{Debug, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::manifest;
use crate::manifest::Manifest;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BuildSystemType {
    Maven,
    Gradle,
    Gallade
}

pub trait BuildSystem: Debug {
    fn detect(&self, path: &Path) -> bool;
    fn get_build_file(&self, path: &Path) -> PathBuf;
    fn get_dependencies(&self, path: &Path) -> anyhow::Result<Vec<String>>;
    fn system_type(&self) -> BuildSystemType;
}

#[derive(Debug)]
pub struct MavenBuildSystem;

impl BuildSystem for MavenBuildSystem {

    // static method
    fn detect(&self, path: &Path) -> bool {
        self.get_build_file(path).exists()
    }

    fn get_build_file(&self, path: &Path) -> PathBuf {
        path.join("pom.xml")
    }

    fn get_dependencies(&self, path: &Path) -> anyhow::Result<Vec<String>> {
        todo!("implement maven dependency parsing")
    }

    fn system_type(&self) -> BuildSystemType {
        BuildSystemType::Maven
    }
}

#[derive(Debug)]
pub struct GradleBuildSystem;

impl BuildSystem for GradleBuildSystem {
    fn detect(&self, path: &Path) -> bool {
        self.get_build_file(path).exists()
    }

    fn get_build_file(&self, path: &Path) -> PathBuf {
        path.join("build.gradle")
    }

    fn get_dependencies(&self, path: &Path) -> anyhow::Result<Vec<String>> {
        todo!("implement gradle dependency parsing")
    }

    fn system_type(&self) -> BuildSystemType {
        BuildSystemType::Gradle
    }
}

#[derive(Debug)]
pub struct GalladeBuildSystem;

impl BuildSystem for GalladeBuildSystem {
    fn detect(&self, path: &Path) -> bool {
        self.get_build_file(path).exists()
    }

    fn get_build_file(&self, path: &Path) -> PathBuf {
        path.join("gallade.toml")
    }

    fn get_dependencies(&self, path: &Path) -> anyhow::Result<Vec<String>> {
        let manifest = Manifest::load(&self.get_build_file(path))?;
        let mut deps = Vec::<String>::new();
        for (name, spec) in manifest.deps {
            match spec {
                manifest::DepSpec::Simple(version) => {
                    deps.push(format!("{}:{}", name, version));
                },
                manifest::DepSpec::Detailed { version, .. } => {
                    deps.push(format!("{}:{}", name, version));
                }
            }
        }

        Ok(deps)
    }

    fn system_type(&self) -> BuildSystemType {
        BuildSystemType::Gallade
    }
}

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    build_system: Arc<dyn BuildSystem>,
}

impl Project {
    pub fn find() -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut current = current_dir.as_path();

        let build_systems: Vec<Arc<dyn BuildSystem>> = vec![
            Arc::new(GalladeBuildSystem),
            Arc::new(MavenBuildSystem)
        ];

        loop {
            for build_system in &build_systems {
                if build_system.detect(current) {
                    return Ok(Self {
                        root: current.to_path_buf(),
                        build_system: build_system.clone(),
                    });
                }
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                anyhow::bail!("couldn't find a supported build file in the current directory or any parent directories");
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn gallade_dir(&self) -> PathBuf {
        self.root.join(".gallade")
    }

    pub fn repository_dir(&self) -> PathBuf {
        self.gallade_dir().join("repository")
    }

    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        fs::create_dir_all(self.repository_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_maven_detection() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("pom.xml")).unwrap();
        assert!(MavenBuildSystem::detect(&MavenBuildSystem{}, temp.path()));
    }

    #[test]
    fn test_project_dirs() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("pom.xml")).unwrap();

        let project = Project {
            root: temp.path().to_path_buf(),
            build_system: Arc::new(MavenBuildSystem),
        };

        assert_eq!(project.gallade_dir(), temp.path().join(".gallade"));
        assert_eq!(project.repository_dir(), temp.path().join(".gallade/repository"));
    }
}
