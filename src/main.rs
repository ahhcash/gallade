mod coordinates;
mod projects;
mod repository;
mod download;
mod resolver;
mod version;
mod lockfile;
mod prune;
mod manifest;
mod classpath;
mod build;
mod init;

use clap::{Parser, Subcommand};
use coordinates::Coordinate;
use download::RepositoryManager;
use projects::Project;
use repository::Repository;
use resolver::DependencyResolver;
use std::collections::HashSet;
use crate::lockfile::Lockfile;

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
    Del {
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
    Build {
        #[arg(short, long)]
        debug: bool,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Init {
        name: String,
        #[arg(short, long)]
        main_class: Option<String>,
        #[arg(short, long)]
        java_version: Option<String>,
    },
}

fn print_tree(
    coord: &Coordinate,
    version: &str,
    seen: &mut HashSet<String>,
    depth: usize,
    detailed: bool,
) {
    let key = format!("{}:{}", coord, version);
    let prefix = "  ".repeat(depth);

    if detailed {
        println!("{}{}:{}", prefix, coord, version);
    } else {
        println!("{}{}", prefix, coord);
    }

    seen.insert(key);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Parse CLI commands first, before any project setup
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { name, main_class, java_version } => {
            // Init command doesn't need existing project
            let initializer = init::ProjectInitializer::new(
                name.clone(),
                main_class.clone(),
                java_version.clone(),
            );

            initializer.init(&std::env::current_dir().unwrap().join(name))?;
            println!("✨ initialized new gallade project: {}", name);
            return Ok(());
        }
        _ => {
            let project = Project::find()?;
            project.ensure_dirs()?;

            let repo = Repository::new(project.repository_dir());
            let manager = RepositoryManager::new()?;
            let resolver = DependencyResolver::new(repo.clone(), manager.clone());

            match &cli.command {
                Commands::Add { coordinate, dev } => {
                    let coord = Coordinate::parse(coordinate)?;
                    println!("resolving dependency {} and its dependencies...", coord);

                    let version = if let Some(v) = coord.version.clone() {
                        v
                    } else {
                        let versions = manager.search_versions(&coord).await?;
                        if versions.is_empty() {
                            anyhow::bail!("no versions found for {}", coord);
                        }
                        versions[0].clone()
                    };

                    let graph = resolver.resolve(&coord, &version).await?;

                    println!("\nResolved dependency tree:");
                    let mut seen = HashSet::new();
                    print_tree(&coord, &version, &mut seen, 0, true);

                    println!("\nSuccessfully added {} and its dependencies", coord);
                    if *dev {
                        println!("Added as a development dependency");
                    }

                    let lockfile_path = project.gallade_dir().join("gallade.lock");
                    let mut lockfile = Lockfile::read(&lockfile_path)?;

                    lockfile.merge_graph(&graph, &manager).await?;
                    lockfile.write(&project.gallade_dir().join("gallade.lock"))?;

                    println!("\nSuccessfully updated gallade.lock");
                }

                Commands::Del { coordinate } => {
                    let coord = Coordinate::parse(coordinate)?;

                    let lockfile_path = project.gallade_dir().join("gallade.lock");
                    let mut lockfile = if lockfile_path.exists() {
                        Lockfile::read(&lockfile_path)?
                    } else {
                        anyhow::bail!("no gallade.lock found - nothing to remove");
                    };

                    let previous_deps: HashSet<String> = lockfile.deps.keys().cloned().collect();

                    if let Err(e) = resolver.remove(coord.clone(), &mut lockfile) {
                        anyhow::bail!("failed to remove {}: {}", coordinate, e);
                    }

                    lockfile.write(&lockfile_path)?;

                    let current_deps: HashSet<String> = lockfile.deps.keys().cloned().collect();
                    let removed_deps = previous_deps.difference(&current_deps);

                    let mut cleaned_count = 0;
                    for dep_str in removed_deps {
                        if let Ok(dep_coord) = Coordinate::parse(dep_str) {
                            if let Some(version) = &dep_coord.version {
                                repo.remove_artifacts(&dep_coord, version)?;
                                cleaned_count += 1;
                            } else {
                                for version in repo.list_versions(&dep_coord)? {
                                    repo.remove_artifacts(&dep_coord, &version)?;
                                    cleaned_count += 1;
                                }
                            }
                        }
                    }

                    if cleaned_count > 0 {
                        println!("Removed {} and {} dependent packages", coordinate, cleaned_count - 1);
                    } else {
                        println!("Removed {}", coordinate);
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

                Commands::Build { debug, args } => {
                    let manifest = manifest::Manifest::load(&project.root().join("gallade.toml"))?;
                    let builder = build::Builder::new(manifest, repo);

                    builder.build(build::BuildOptions {
                        args: args.clone(),
                        debug: *debug,
                    })?;
                }

                Commands::Init { .. } => unreachable!(),
            }
        }
    }
    Ok(())
}