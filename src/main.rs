mod coordinates;
mod projects;
mod repository;
mod download;
mod resolver;

use clap::{Parser, Subcommand};
use coordinates::Coordinate;
use download::RepositoryManager;
use projects::Project;
use repository::Repository;
use resolver::DependencyResolver;
use std::collections::HashSet;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        coordinate: String,
        #[arg(short, long)]
        dev: bool,
    },
    Remove {
        coordinate: String
    },
    List {
        coordinate: String
    },
    Search {
        coordinate: String
    },
    Tree {
        #[arg(short, long)]
        detailed: bool,
    },
}

// Helper function to print a dependency tree
fn print_tree(
    coord: &Coordinate,
    version: &str,
    seen: &mut HashSet<String>,
    depth: usize,
    detailed: bool,
) {
    let key = format!("{}:{}", coord, version);
    let prefix = "  ".repeat(depth);

    // Print with or without version based on detailed flag
    if detailed {
        println!("{}{}:{}", prefix, coord, version);
    } else {
        println!("{}{}", prefix, coord);
    }

    // Track what we've seen to handle cycles
    seen.insert(key);

    // Here we'd traverse child dependencies from our graph
    // (We'll implement this when we store the full graph)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Set up our core components
    let project = Project::find()?;
    project.ensure_dirs()?;

    let repo = Repository::new(project.repository_dir());
    let manager = RepositoryManager::new()?;
    let resolver = DependencyResolver::new(repo.clone(), manager.clone());

    let cli = Cli::parse();
    match &cli.command {
        Commands::Add { coordinate, dev } => {
            let coord = Coordinate::parse(coordinate)?;
            println!("resolving dependency {} and its dependencies...", coord);

            // Get the version (either specified or latest)
            let version = if let Some(v) = coord.version.clone() {
                v
            } else {
                let versions = manager.search_versions(&coord).await?;
                if versions.is_empty() {
                    anyhow::bail!("no versions found for {}", coord);
                }
                versions[0].clone()
            };

            // Resolve the complete dependency graph
            let graph = resolver.resolve(&coord, &version).await?;

            println!("\nResolved dependency tree:");
            let mut seen = HashSet::new();
            print_tree(&coord, &version, &mut seen, 0, true);

            println!("\nSuccessfully added {} version {} and its dependencies", coord, version);
            if *dev {
                println!("Added as a development dependency");
            }
        }

        Commands::Remove { coordinate } => {
            let coord = Coordinate::parse(coordinate)?;
            if let Some(version) = &coord.version {
                repo.remove_artifacts(&coord, version)?;
                println!("removed {} version {}", coord, version);
            } else {
                for version in repo.list_versions(&coord)? {
                    repo.remove_artifacts(&coord, &version)?;
                }
                println!("removed all versions of {}", coord);
            }
        }

        Commands::List { coordinate } => {
            let coord = Coordinate::parse(coordinate)?;
            let versions = repo.list_versions(&coord)?;
            if versions.is_empty() {
                println!("no versions of {} found locally", coord);
            } else {
                println!("installed versions of {}:", coord);
                for version in versions {
                    println!("  {}", version);
                }
            }
        }

        Commands::Search { coordinate } => {
            let coord = Coordinate::parse(coordinate)?;
            let versions = manager.search_versions(&coord).await?;
            if versions.is_empty() {
                println!("no versions found for {}", coord);
            } else {
                println!("available versions of {}:", coord);
                for version in versions {
                    println!("  {}", version);
                }
            }
        }

        Commands::Tree { detailed } => {
            println!("dependency tree for current project:");
            // TODO: Read project's direct dependencies and show full tree
            println!("(tree visualization coming soon)");
        }
    }
    Ok(())
}