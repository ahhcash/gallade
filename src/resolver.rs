use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use log::__private_api::loc;
use serde::Deserialize;

use crate::coordinates::Coordinate;
use crate::download::RepositoryManager;
use crate::lockfile::Lockfile;
use crate::prune::DependencyPruner;
use crate::repository::{Repository, ArtifactKind};
use crate::version::{MavenVersion, VersionReq};

#[derive(Debug, Clone)]
pub struct DependencyRequest {
    pub coordinate: Coordinate,
    pub version_req: VersionReq,
    pub scope: Option<String>,
    pub depth: usize
}


#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub resolved: HashMap<Coordinate, MavenVersion>,
    requirements: HashMap<Coordinate, Vec<(VersionReq, usize)>>,
    pub edges: HashMap<Coordinate, HashSet<Coordinate>>,
}

#[derive(Debug, Default)]
pub struct ReverseDependencyGraph {
    dependents: HashMap<Coordinate, HashSet<Coordinate>>
}

impl ReverseDependencyGraph {
    pub fn from_lockfile(lockfile: &Lockfile) -> Self {
        let mut graph = Self::default();

        for (coord, info) in &lockfile.deps {
            let is_depended_by = Coordinate::parse(coord).unwrap();
            for dep in &info.deps {
                let dependency = Coordinate::parse(dep).unwrap();
                graph.add_edge(dependency, is_depended_by.clone());
            }
        }

        graph
    }

    fn add_edge(&mut self, target: Coordinate, dependent: Coordinate) {
        self.dependents
            .entry(target)
            .or_default()
            .insert(dependent);
    }

    pub fn has_dependents(&self, coord: &Coordinate) -> bool {
        self.dependents
            .get(coord)
            .map(|deps| !deps.is_empty())
            .unwrap_or(false)
    }

    pub fn get_dependents(&self, coord: &Coordinate) -> HashSet<Coordinate> {
        self.dependents
            .get(coord)
            .cloned()
            .unwrap_or_default()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_requirement(&mut self, coord: &Coordinate, req: VersionReq, depth: usize) {
        self.requirements
            .entry(coord.clone())
            .or_default()
            .push((req, depth));
    }

    pub fn add_edge(&mut self, from: &Coordinate, to: &Coordinate) {
        self.edges
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
    }

    pub fn check_version_compatibility(&self, coord: &Coordinate, version: &MavenVersion) -> bool {
        if let Some(reqs) = self.requirements.get(coord) {
            let mut sorted_reqs = reqs.clone();
            sorted_reqs.sort_by_key(|(_, depth)| *depth);

            let (nearest_req, _) = &sorted_reqs[0];

            nearest_req.matches(version)
        } else {
            true
        }
    }

    pub fn add_resolution(&mut self, coord: &Coordinate, version: MavenVersion) {
        self.resolved.insert(coord.clone(), version);
    }
}

pub trait MetadataParser {
    fn parse_dependencies(&self, content: &str) -> anyhow::Result<Vec<DependencyRequest>>;
}

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
            if dep.scope.as_deref() == Some("test") {
                continue;
            }

            let coord = Coordinate {
                namespace: dep.group_id,
                name: dep.artifact_id,
                version: None,
            };

            let version_req = match dep.version {
                Some(v) => VersionReq::parse(&v)?,
                None => VersionReq::Latest,
            };

            requests.push(DependencyRequest {
                coordinate: coord,
                version_req,
                scope: dep.scope,
                depth: 0
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
        queue.push_back((root_coord.clone(), root_version.clone(), 0));

        while let Some((coord, version, depth)) = queue.pop_front() {
            let key = format!("{}:{:?}", coord, version);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            if !self.repo.has_artifact(&coord, &version.to_string(), ArtifactKind::Binary) {
                let jar = self.manager.download_jar(&coord, &version.to_string()).await?;
                self.repo.store_artifact(&coord, &version.to_string(), ArtifactKind::Binary, jar).await?;
            }

            let metadata = if self.repo.has_artifact(&coord, &version.to_string(), ArtifactKind::Metadata) {
                String::from_utf8(self.repo.load_artifact(&coord, &version.to_string(), ArtifactKind::Metadata)?)?
            } else {
                let metadata = self.manager.download_metadata(&coord, &version.to_string()).await?;
                self.repo.store_artifact(&coord, &version.to_string(), ArtifactKind::Metadata, metadata.as_bytes()).await?;
                metadata
            };

            let mut deps = self.parser.parse_dependencies(&metadata)?;
            for dep in &mut deps {
                dep.depth = depth + 1;
            }

            for dep in deps {
                graph.add_requirement(&dep.coordinate, dep.version_req.clone(), dep.depth);
                graph.add_edge(&coord, &dep.coordinate);

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
                    graph.add_resolution(&dep.coordinate.clone(), v.clone());
                    queue.push_back((dep.coordinate.clone(), v, dep.depth));
                } else {
                    anyhow::bail!("no compatible version found for {} with version: {:?}", dep.coordinate, dep.version_req);
                }
            }
        }

        graph.add_resolution(root_coord, root_version);

        Ok(graph)
    }

    pub fn remove(&self, coord: Coordinate, lockfile: &mut Lockfile) -> anyhow::Result<()> {
        let mut pruner = DependencyPruner::new();

        pruner.mark_tree(&coord, lockfile, true);

        for (coord_str, _) in lockfile.deps.iter() {
            if let Ok(c) = Coordinate::parse(coord_str) {
                if c != coord {
                    pruner.mark_tree(&coord, lockfile,  false);
                }
            }
        }

        lockfile.deps.remove(&coord.to_string());

        let to_remove = pruner.get_removable();

        for coord in to_remove {
            lockfile.deps.remove(&coord.to_string());
        }

        Ok(())
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