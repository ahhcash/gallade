use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use crate::download::RepositoryManager;
use crate::resolver::DependencyGraph;

#[derive(Debug, Deserialize, Serialize)]
pub struct Lockfile {
    version: u32,
    deps: HashMap<String, PackageInfo>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PackageInfo {
    version: String,
    repository: String,
    integrity: String,
    deps: Vec<String>
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            version: 1,
            deps: HashMap::new()
        }
    }

    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub async fn from_graph(
        graph: &DependencyGraph,
        repo_manager: &RepositoryManager
    ) -> anyhow::Result<Self> {
        let mut lockfile = Self::new();

        for (coord, version) in graph.resolved.iter() {
            // Download jar to compute hash if needed
            let jar = repo_manager.download_jar(coord, &version.to_string()).await?;

            let mut hasher = Sha256::new();
            hasher.update(&jar);
            let hash_bytes = hasher.finalize();
            let hash = format!("sha256:{}", hex::encode(hash_bytes));

            let repo_name = "maven-central";

            let deps = graph.edges.get(coord)
                .map(|deps| {
                    deps.iter()
                        .map(|d| d.to_string())
                        .collect()
                })
                .unwrap_or_default();

            lockfile.deps.insert(
                coord.to_string(),
                PackageInfo {
                    version: version.to_string(),
                    repository: repo_name.to_string(),
                    integrity: hash,
                    deps,
                }
            );
        }

        Ok(lockfile)
    }
}

