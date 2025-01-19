use std::path::{PathBuf};
use std::fs;

use crate::coordinates::Coordinate;

#[derive(Clone)]
pub struct Repository {
    root: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum ArtifactKind {
    Binary,
    Metadata,
}

impl Repository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn get_artifact_path(&self, coord: &Coordinate, version: &str, kind: ArtifactKind) -> PathBuf {
        let extension = match kind {
            ArtifactKind::Binary => "jar",
            ArtifactKind::Metadata => "pom",
        };

        self.root
            .join(coord.to_path())
            .join(version)
            .join(format!("{}-{}.{}", coord.name, version, extension))
    }

    pub async fn store_artifact(
        &self,
        coord: &Coordinate,
        version: &str,
        kind: ArtifactKind,
        content: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let path = self.get_artifact_path(coord, version, kind);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, content)?;
        Ok(())
    }

    pub fn has_artifact(&self, coord: &Coordinate, version: &str, kind: ArtifactKind) -> bool {
        self.get_artifact_path(coord, version, kind).exists()
    }

    pub fn load_artifact(
        &self,
        coord: &Coordinate,
        version: &str,
        kind: ArtifactKind,
    ) -> anyhow::Result<Vec<u8>> {
        let path = self.get_artifact_path(coord, version, kind);
        Ok(fs::read(path)?)
    }

    pub fn remove_artifacts(&self, coord: &Coordinate, version: &str) -> anyhow::Result<()> {
        let version_dir = self.root
            .join(coord.to_path())
            .join(version);

        if version_dir.exists() {
            fs::remove_dir_all(version_dir)?;
        }
        Ok(())
    }

    pub fn list_versions(&self, coord: &Coordinate) -> anyhow::Result<Vec<String>> {
        let coord_dir = self.root.join(coord.to_path());

        if !coord_dir.exists() {
            return Ok(vec![]);
        }

        let mut versions = vec![];
        for entry in fs::read_dir(coord_dir)? {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(version) = entry.file_name().to_str() {
                        versions.push(version.to_string());
                    }
                }
            }
        }

        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_artifact_paths() {
        let temp = TempDir::new().unwrap();
        let repo = Repository::new(temp.path().to_path_buf());

        let coord = Coordinate::parse("com.google.guava:guava").unwrap();
        let version = "31.1-jre";

        let jar_path = repo.get_artifact_path(&coord, version, ArtifactKind::Binary);
        assert_eq!(
            jar_path.strip_prefix(temp.path()).unwrap(),
            Path::new("com/google/guava/guava/31.1-jre/guava-31.1-jre.jar")
        );

        let pom_path = repo.get_artifact_path(&coord, version, ArtifactKind::Metadata);
        assert_eq!(
            pom_path.strip_prefix(temp.path()).unwrap(),
            Path::new("com/google/guava/guava/31.1-jre/guava-31.1-jre.pom")
        );
    }

    #[test]
    fn test_store_and_load() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let repo = Repository::new(temp.path().to_path_buf());

        let coord = Coordinate::parse("org.slf4j:slf4j-api").unwrap();
        let version = "1.7.36";
        let content = b"test content";

        tokio_test::block_on(repo.store_artifact(
            &coord,
            version,
            ArtifactKind::Binary,
            content
        ))?;

        assert!(repo.has_artifact(&coord, version, ArtifactKind::Binary));
        assert_eq!(
            repo.load_artifact(&coord, version, ArtifactKind::Binary)?,
            content
        );

        Ok(())
    }
}