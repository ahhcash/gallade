use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use tempfile::NamedTempFile;
use crate::coordinates::Coordinate;
use crate::download::RepositoryManager;
use crate::resolver::DependencyGraph;

#[derive(Debug, Deserialize, Serialize)]
pub struct Lockfile {
    version: u32,
    pub deps: HashMap<String, PackageInfo>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PackageInfo {
    version: String,
    repository: String,
    integrity: String, pub deps: Vec<String>
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            version: 1,
            deps: HashMap::new()
        }
    }

    fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn read(path: &Path) -> anyhow::Result<Self> {
        Self::ensure_parent_dir(path)?;

        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        Self::ensure_parent_dir(path)?;

        let dir = path.parent().unwrap_or(Path::new("."));
        let mut temp_file = NamedTempFile::new_in(dir)?;

        let content = serde_json::to_string_pretty(self)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;

        temp_file.persist(path)?;

        Ok(())
    }

    pub async fn merge_graph(
        &mut self,
        graph: &DependencyGraph,
        repo_manager: &RepositoryManager
    ) -> anyhow::Result<()> {
        for (coord, version) in graph.resolved.iter() {
            // If this exact version is already in the lockfile, skip recomputing hash
            let key = coord.to_string();
            if let Some(existing) = self.deps.get(&key) {
                if existing.version == version.to_string() {
                    continue;
                }
            }

            let jar = repo_manager.download_jar(coord, &version.to_string()).await?;

            let mut hasher = Sha256::new();
            hasher.update(&jar);
            let hash_bytes = hasher.finalize();
            let hash = format!("sha256:{}", hex::encode(hash_bytes));

            let repo_name = repo_manager.fetch_source_repo(coord).await?;

            let deps = Self::get_stringified_edges(graph, coord);

            self.deps.insert(
                key,
                PackageInfo {
                    version: version.to_string(),
                    repository: repo_name.to_string(),
                    integrity: hash,
                    deps,
                }
            );
        }

        Ok(())
    }

    fn get_stringified_edges(graph: &DependencyGraph, coord: &Coordinate) -> Vec<String> {
        graph.edges.get(coord)
            .map(|dep| {
                dep.iter()
                    .map(|d| d.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lockfile_atomic_write() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let lockfile_path = temp.path().join("test.lock");

        // Create a lockfile with some data
        let mut lockfile = Lockfile::new();
        lockfile.deps.insert(
            "test:package".to_string(),
            PackageInfo {
                version: "1.0.0".to_string(),
                repository: "test-repo".to_string(),
                integrity: "sha256:test".to_string(),
                deps: vec![]
            }
        );

        // Write it atomically
        lockfile.write(&lockfile_path)?;

        // Read it back
        let read_lockfile = Lockfile::read(&lockfile_path)?;

        // Verify contents
        assert_eq!(
            read_lockfile.deps.get("test:package").unwrap().version,
            "1.0.0"
        );

        Ok(())
    }

    #[test]
    fn test_lockfile_directory_creation() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let nested_path = temp.path()
            .join("deeply")
            .join("nested")
            .join("dirs")
            .join("lock.json");

        // This should create all parent directories
        let lockfile = Lockfile::new();
        lockfile.write(&nested_path)?;

        // Verify directories were created
        assert!(nested_path.parent().unwrap().exists());

        Ok(())
    }
}