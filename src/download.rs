use std::collections::HashSet;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use reqwest::Client;

use crate::coordinates::Coordinate;

#[derive(Debug, Deserialize, Serialize)]
struct MavenResponse {
    response: SearchResponse,
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchResponse {
    docs: Vec<ArtifactDoc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArtifactDoc {
    pub g: String,
    pub a: String,
    pub v: String,
    pub p: String,
    pub timestamp: i64,
}

#[async_trait::async_trait]
pub trait ArtifactRepository: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, coord: &Coordinate) -> anyhow::Result<Vec<String>>;
    async fn fetch_jar(&self, coord: &Coordinate, version: &str) -> anyhow::Result<Vec<u8>>;
    async fn fetch_metadata(&self, coord: &Coordinate, version: &str) -> anyhow::Result<String>;
}


pub struct MavenCentral {
    client: Client,
}

impl MavenCentral {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent("gallade/0.1.0")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { client })
    }

    async fn search_artifacts(&self, coord: &Coordinate) -> anyhow::Result<Vec<ArtifactDoc>> {
        let url = format!(
            "https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&core=gav&rows=20&wt=json",
            urlencoding::encode(&coord.namespace),
            urlencoding::encode(&coord.name)
        );

        let response = self.client
            .get(&url)
            .send()
            .await?
            .json::<MavenResponse>()
            .await?;

        Ok(response.response.docs)
    }
}

#[async_trait::async_trait]
impl ArtifactRepository for MavenCentral {
    fn name(&self) -> &str {
        "MavenCentral"
    }

    async fn search(&self, coord: &Coordinate) -> anyhow::Result<Vec<String>> {
        let artifacts = self.search_artifacts(coord).await?;
        Ok(artifacts.into_iter().map(|doc| doc.v).collect())
    }

    async fn fetch_jar(&self, coord: &Coordinate, version: &str) -> anyhow::Result<Vec<u8>> {
        let jar_path = format!("{}-{}.jar", coord.name, version);
        let url = format!(
            "https://search.maven.org/remotecontent?filepath={}/{}/{}/{}",
            coord.namespace.replace('.', "/"),
            coord.name,
            version,
            jar_path
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("failed to download jar: HTTP {}", response.status());
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn fetch_metadata(&self, coord: &Coordinate, version: &str) -> anyhow::Result<String> {
        let pom_path = format!("{}-{}.pom", coord.name, version);
        let url = format!(
            "https://search.maven.org/remotecontent?filepath={}/{}/{}/{}",
            coord.namespace.replace('.', "/"),
            coord.name,
            version,
            pom_path
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("failed to download pom: HTTP {}", response.status());
        }

        Ok(response.text().await?)
    }
}

#[derive(Clone)]
pub struct RepositoryManager {
    repositories: Vec<Arc<dyn ArtifactRepository + Send + Sync>>,
}

impl RepositoryManager {
    pub fn new() -> anyhow::Result<Self> {
        let repositories: Vec<Arc<dyn ArtifactRepository + Send + Sync>> = vec![
            Arc::new(MavenCentral::new()?)
        ];

        Ok(Self { repositories })
    }

    // Searches all repositories and returns consolidated results
    pub async fn search_versions(&self, coord: &Coordinate) -> anyhow::Result<Vec<String>> {
        let mut versions = HashSet::new();

        for repo in &self.repositories {
            if let Ok(repo_versions) = repo.search(coord).await {
                versions.extend(repo_versions);
            }
        }

        Ok(versions.into_iter().collect())
    }

    pub async fn download_jar(&self, coord: &Coordinate, version: &str) -> anyhow::Result<Vec<u8>> {
        for repo in &self.repositories {
            match repo.fetch_jar(coord, version).await {
                Ok(bytes) => return Ok(bytes),
                Err(_) => continue,
            }
        }
        anyhow::bail!("could not download jar from any repository")
    }

    // Same for metadata
    pub async fn download_metadata(&self, coord: &Coordinate, version: &str) -> anyhow::Result<String> {
        for repo in &self.repositories {
            match repo.fetch_metadata(coord, version).await {
                Ok(text) => return Ok(text),
                Err(_) => continue,
            }
        }
        anyhow::bail!("could not download metadata from any repository")
    }

    pub async fn fetch_source_repo(&self, coord: &Coordinate) -> anyhow::Result<String> {
        for repo in &self.repositories {
           if repo.search(coord).await.is_ok() {
               return Ok(repo.name().to_string())
           }
        }

        anyhow::bail!("no repository found for coordinate: {}", coord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_maven_central_search() -> anyhow::Result<()> {
        let maven = MavenCentral::new()?;
        let coord = Coordinate::parse("org.slf4j:slf4j-api")?;

        let versions = maven.search(&coord).await?;
        assert!(!versions.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_maven_central_fetch() -> anyhow::Result<()> {
        let maven = MavenCentral::new()?;
        let coord = Coordinate::parse("org.slf4j:slf4j-api")?;

        let jar = maven.fetch_jar(&coord, "1.7.36").await?;
        assert!(!jar.is_empty());

        let pom = maven.fetch_metadata(&coord, "1.7.36").await?;
        assert!(!pom.is_empty());

        Ok(())
    }
}