use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug)]
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

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        coordinate: String
    },
    Remove {
        coordinate: String
    }
}

async fn fetch_artifact_info(coord: &MavenCoordinate) -> Result<Vec<ArtifactDoc>, anyhow::Error> {
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Add { coordinate } => {
            let coord = MavenCoordinate::parse(coordinate)?;
            let artifacts = fetch_artifact_info(&coord).await?;

        }

        Commands::Remove { coordinate } => {
            println!("removing dependency {}", coordinate);
        }
    }
    Ok(())
}