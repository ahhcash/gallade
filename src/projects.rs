use std::path::{Path, PathBuf};

pub trait BuildSystem {
    fn detect(path: &Path) -> bool;
    fn get_build_file(path: &Path) -> PathBuf;
    fn get_dependencies(path: &Path) -> anyhow::Result<Vec<String>>;
}

pub struct MavenBuildSystem;

impl BuildSystem for MavenBuildSystem {
    fn detect(path: &Path) -> bool {
        path.join("pom.xml").exists()
    }

    fn get_build_file(path: &Path) -> PathBuf {
        path.join("pom.xml")
    }

    fn get_dependencies(path: &Path) -> anyhow::Result<Vec<String>> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    build_system: Box<dyn BuildSystem>,
}

impl Project {
    pub fn find() -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut current = current_dir.as_path();

        let build_systems: Vec<Box<dyn BuildSystem>> = vec![
            Box::new(MavenBuildSystem)
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

        assert!(MavenBuildSystem::detect(temp.path()));
    }

    #[test]
    fn test_project_dirs() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("pom.xml")).unwrap();

        let project = Project {
            root: temp.path().to_path_buf(),
            build_system: Box::new(MavenBuildSystem),
        };

        assert_eq!(project.gallade_dir(), temp.path().join(".gallade"));
        assert_eq!(project.repository_dir(), temp.path().join(".gallade/repository"));
    }
}
