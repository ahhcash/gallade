use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug, Clone)]
struct MavenCoordinate {
    group_id: String,
    artifact_id: String,
    version: Option<String>,
}

impl MavenCoordinate {
    fn parse(coord: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = coord.split(':').collect();
        match parts.len() {
            2 => Ok(Self {
                group_id: parts[0].to_string(),
                artifact_id: parts[1].to_string(),
                version: None,
            }),
            3 => Ok(Self {
                group_id: parts[0].to_string(),
                artifact_id: parts[1].to_string(),
                version: Some(parts[2].to_string()),
            }),
            _ => anyhow::bail!("invalid maven coordinate format")
        }
    }

    fn to_path(&self) -> String {
        format!(
            "{}/{}",
            self.group_id.replace('.', "/"),
            self.artifact_id
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct MavenResponse {
    response: SearchResponse,
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchResponse {
    docs: Vec<ArtifactDoc>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ArtifactDoc {
    g: String,
    a: String,
    v: String,
    p: String,
    timestamp: i64,
}

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

#[derive(Debug, Deserialize, Clone)]
struct Dependency {
    #[serde(rename = "groupId")]
    group_id: String,
    #[serde(rename = "artifactId")]
    artifact_id: String,
    version: Option<String>,
    #[serde(default)]
    scope: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        coordinate: String
    },
    Remove {
        coordinate: String
    }
}

async fn fetch_artifact_info(coord: &MavenCoordinate) -> anyhow::Result<Vec<ArtifactDoc>> {
    let client = reqwest::Client::builder()
        .user_agent("gallade/0.1.0")
        .build()?;
    let url = format!(
        "https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&core=gav&rows=20&wt=json",
        urlencoding::encode(&coord.group_id),
        urlencoding::encode(&coord.artifact_id)
    );

    let response = client.get(&url).send().await?.json::<MavenResponse>().await?;
    Ok(response.response.docs)
}

async fn download_jar(coord: &MavenCoordinate, version: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("gallade/0.1.0")
        .build()?;    let jar_path = format!("{}-{}.jar", coord.artifact_id, version);
    let url = format!(
        "https://search.maven.org/remotecontent?filepath={}/{}/{}/{}",
        coord.group_id.replace('.', "/"),
        coord.artifact_id,
        version,
        jar_path
    );

    let local_path = get_local_path(coord, version, &jar_path)?;

    // create parent dirs
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = client.get(&url).send().await?;
    let bytes = response.bytes().await?;
    fs::write(&local_path, bytes)?;

    Ok(())
}

fn get_local_path(coord: &MavenCoordinate, version: &str, filename: &str) -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")?;
    let path = format!(
        "{}/.m2/repository/{}/{}/{}/{}",
        home,
        coord.group_id.replace('.', "/"),
        coord.artifact_id,
        version,
        filename
    );
    Ok(PathBuf::from(path))
}

async fn download_pom(coord: &MavenCoordinate, version: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("gallade/0.1.0")
        .build()?;    let pom_path = format!("{}-{}.pom", coord.artifact_id, version);
    let url = format!(
        "https://search.maven.org/remotecontent?filepath={}/{}/{}/{}",
        coord.group_id.replace('.', "/"),
        coord.artifact_id,
        version,
        pom_path
    );

    let local_path = get_local_path(coord, version, &pom_path)?;

    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = client.get(&url).send().await?;
    let text = response.text().await?;
    fs::write(&local_path, &text)?;

    Ok(text)
}

async fn get_latest_version(coord: &MavenCoordinate) -> anyhow::Result<String> {
    let artifact_info = fetch_artifact_info(coord).await?;
    if artifact_info.is_empty() {
        anyhow::bail!("could not find artifact for {}:{}", coord.group_id, coord.group_id)
    }

    Ok(artifact_info[0].v.clone())
}

async fn resolve_dependencies(coord: &MavenCoordinate, version: &str) -> anyhow::Result<()> {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    // pushing back initial dependency
    queue.push_back((coord.clone(), version.to_string()));
    // doing some BFS
    while let Some((dep_coord, dep_version)) = queue.pop_front() {
        let key = format!("{}:{}:{}", dep_coord.group_id, dep_coord.artifact_id, dep_version);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        download_jar(&dep_coord, &*dep_version).await?;
        let pom = download_pom(&dep_coord, &*dep_version).await?;
        let project: Project = from_str(&pom)?;
        for dep in project.dependencies.dependency {
            if dep.scope != "test" {
                let dep_coord = &MavenCoordinate {
                    group_id: dep.group_id,
                    artifact_id: dep.artifact_id,
                    version: dep.version.clone()
                };

                let resolved_version = match dep.version {
                    Some(v) => v,
                    None => get_latest_version(dep_coord).await?
                };

                queue.push_back((dep_coord.clone(), resolved_version))
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Add { coordinate } => {
            let coord = MavenCoordinate::parse(coordinate)?;
            let artifacts = fetch_artifact_info(&coord).await?;

            if artifacts.is_empty() {
                anyhow::bail!("no artifacts found for {}", coordinate);
            }

            let version = match &coord.version {
                Some(v) => v.clone(),
                None => artifacts[0].v.clone()
            };

            resolve_dependencies(&coord, &version).await?;
            println!("resolved all dependencies");
        }

        Commands::Remove { coordinate } => {
            println!("removing dependency {}", coordinate);
        }
    }
    Ok(())
}