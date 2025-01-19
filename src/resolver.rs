use std::collections::{HashMap, HashSet, VecDeque};
use serde::Deserialize;

use crate::coordinates::Coordinate;
use crate::download::RepositoryManager;
use crate::repository::{Repository, ArtifactKind};
use crate::version::{MavenVersion, VersionReq};

#[derive(Debug, Clone)]
pub struct DependencyRequest {
    pub coordinate: Coordinate,
    pub version_req: VersionReq,  // Now using our custom VersionReq
    pub scope: Option<String>,
}

#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub(crate) resolved: HashMap<Coordinate, MavenVersion>,  // Now storing MavenVersion
    requirements: HashMap<Coordinate, HashSet<VersionReq>>,
    pub(crate) edges: HashMap<Coordinate, HashSet<Coordinate>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_requirement(&mut self, coord: &Coordinate, req: VersionReq) {
        self.requirements
            .entry(coord.clone())
            .or_default()
            .insert(req);
    }

    pub fn add_edge(&mut self, from: &Coordinate, to: &Coordinate) {
        self.edges
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
    }

    // Updated to use our version types
    pub fn check_version_compatibility(&self, coord: &Coordinate, version: &MavenVersion) -> bool {
        if let Some(reqs) = self.requirements.get(coord) {
            reqs.iter().all(|req| req.matches(version))
        } else {
            true
        }
    }
}

pub trait MetadataParser {
    fn parse_dependencies(&self, content: &str) -> anyhow::Result<Vec<DependencyRequest>>;
}

// Maven specific parser
pub struct PomParser;

impl MetadataParser for PomParser {
    fn parse_dependencies(&self, content: &str) -> anyhow::Result<Vec<DependencyRequest>> {
        #[derive(Debug, Deserialize)]
        struct Project {
            #[serde(default)]
            dependencies: Dependencies,
        }

        #[derive(Debug, Default, Deserialize)]
        struct Dependencies {
            #[serde(default)]
            dependency: Vec<Dependency>,
        }

        #[derive(Debug, Deserialize)]
        struct Dependency {
            #[serde(rename = "groupId")]
            group_id: String,
            #[serde(rename = "artifactId")]
            artifact_id: String,
            version: Option<String>,
            #[serde(default)]
            scope: Option<String>,
        }

        let project: Project = quick_xml::de::from_str(content)?;
        let mut requests = Vec::new();

        for dep in project.dependencies.dependency {
            // Skip test dependencies
            if dep.scope.as_deref() == Some("test") {
                continue;
            }

            let coord = Coordinate {
                namespace: dep.group_id,
                name: dep.artifact_id,
                version: None,
            };

            // Parse version requirement if present, default to LATEST
            let version_req = match dep.version {
                Some(v) => VersionReq::parse(&v)?,
                None => VersionReq::Latest,
            };

            requests.push(DependencyRequest {
                coordinate: coord,
                version_req,
                scope: dep.scope,
            });
        }

        Ok(requests)
    }
}

pub struct DependencyResolver {
    repo: Repository,
    manager: RepositoryManager,
    parser: Box<dyn MetadataParser>,
}

impl DependencyResolver {
    pub fn new(repo: Repository, manager: RepositoryManager) -> Self {
        Self {
            repo,
            manager,
            parser: Box::new(PomParser),
        }
    }

    pub async fn resolve(&self, root_coord: &Coordinate, version: &str) -> anyhow::Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();

        let root_version = version.parse::<MavenVersion>()?;
        queue.push_back((root_coord.clone(), root_version));

        while let Some((coord, version)) = queue.pop_front() {
            let key = format!("{}:{:?}", coord, version);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            // Download and store the dependency if we don't have it
            if !self.repo.has_artifact(&coord, &version.to_string(), ArtifactKind::Binary) {
                let jar = self.manager.download_jar(&coord, &version.to_string()).await?;
                self.repo.store_artifact(&coord, &version.to_string(), ArtifactKind::Binary, jar).await?;
            }

            // Get or download metadata
            let metadata = if self.repo.has_artifact(&coord, &version.to_string(), ArtifactKind::Metadata) {
                String::from_utf8(self.repo.load_artifact(&coord, &version.to_string(), ArtifactKind::Metadata)?)?
            } else {
                let metadata = self.manager.download_metadata(&coord, &version.to_string()).await?;
                self.repo.store_artifact(&coord, &version.to_string(), ArtifactKind::Metadata, metadata.as_bytes()).await?;
                metadata
            };

            // Parse dependencies from metadata
            let deps = self.parser.parse_dependencies(&metadata)?;

            // Process each dependency
            for dep in deps {
                graph.add_requirement(&dep.coordinate, dep.version_req.clone());
                graph.add_edge(&coord, &dep.coordinate);

                // Find a version that satisfies all requirements
                let available_versions = self.manager.search_versions(&dep.coordinate).await?;
                let mut compatible_version = None;

                for v in available_versions {
                    let maven_version: MavenVersion = v.parse()?;
                    if graph.check_version_compatibility(&dep.coordinate, &maven_version) {
                        compatible_version = Some(maven_version);
                        break;
                    }
                }

                if let Some(v) = compatible_version {
                    queue.push_back((dep.coordinate.clone(), v));
                } else {
                    anyhow::bail!("no compatible version found for {}", dep.coordinate);
                }
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pom_parser() -> anyhow::Result<()> {
        let parser = PomParser;
        let content = r#"
            <project>
                <dependencies>
                    <dependency>
                        <groupId>org.slf4j</groupId>
                        <artifactId>slf4j-api</artifactId>
                        <version>1.7.36</version>
                    </dependency>
                    <dependency>
                        <groupId>junit</groupId>
                        <artifactId>junit</artifactId>
                        <version>4.13.2</version>
                        <scope>test</scope>
                    </dependency>
                </dependencies>
            </project>
        "#;

        let deps = parser.parse_dependencies(content)?;
        assert_eq!(deps.len(), 1); // junit should be skipped
        assert_eq!(deps[0].coordinate.namespace, "org.slf4j");

        Ok(())
    }
}