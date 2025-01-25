use std::process::Command;
use std::time::SystemTime;
use walkdir::WalkDir;
use crate::build::{BuildOptions, Builder};
use crate::classpath;
use crate::classpath::ClasspathBuilder;
use crate::coordinates::Coordinate;
use crate::manifest::Manifest;
use crate::repository::Repository;

pub struct RunOptions {
     pub(crate) debug: bool,
     pub(crate) args: Vec<String>
 }

 pub struct Runner {
     manifest: Manifest,
     repository: Repository
 }

impl Runner {
    pub fn new(manifest: Manifest, repository: Repository) -> Self {
        Self {
            manifest,
            repository
        }
    }

    fn needs_compilation(&self) -> anyhow::Result<bool> {
        let target_dir = std::env::current_dir()?.join("target").join("classes");
        let src_dir = std::env::current_dir()?.join("src").join("main").join("java");

        if !target_dir.exists() {
            return Ok(true);
        }

        let mut latest_src = SystemTime::UNIX_EPOCH;
        for entry in WalkDir::new(src_dir) {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("java") {
                if let Ok(modified) = entry.metadata()?.modified() {
                    if modified > latest_src {
                        latest_src = modified;
                    }
                }
            }
        }

        let mut oldest_class = SystemTime::now();
        for entry in WalkDir::new(&target_dir) {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("class") {
                if let Ok(modified) = entry.metadata()?.modified() {
                    if modified < oldest_class {
                        oldest_class = modified;
                    }
                }
            }
        }

        Ok(latest_src > oldest_class)
    }

    pub fn run(&self, options: RunOptions) -> anyhow::Result<()> {
        // Check if we need to recompile and do so if necessary
        if self.needs_compilation()? {
            let builder = Builder::new(self.manifest.clone(), self.repository.clone());
            builder.build(BuildOptions {
                debug: options.debug,
            })?;
        }

        // Set up the runtime classpath
        let mut cp_builder = ClasspathBuilder::new(self.repository.clone());

        // Add target/classes as the first entry
        let target_dir = std::env::current_dir()?.join("target").join("classes");
        cp_builder.with_dir(target_dir.clone());

        // Add all dependencies
        for (coord_str, dep_spec) in &self.manifest.deps {
            let coord = Coordinate::parse(coord_str)?;
            let version = match dep_spec {
                crate::manifest::DepSpec::Simple(v) => v,
                crate::manifest::DepSpec::Detailed { version, .. } => version,
            };
            cp_builder.with_dep(&coord, version);
        }

        // Build final classpath
        let classpath = cp_builder.build();

        // Resolve and verify main class
        let main_class = classpath::resolve_main_class(
            &target_dir,
            self.manifest.project.main_class.clone()
        )?;

        // Prepare java command
        let mut java = Command::new("java");

        if options.debug {
            java.arg("-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005");
        }

        // Add classpath and main class
        java.args(["-cp", &classpath, &main_class]);

        // Add any user-provided arguments
        java.args(&options.args);

        // Execute and check status
        let status = java.status()?;
        if !status.success() {
            anyhow::bail!("Java process failed with status: {}", status);
        }

        Ok(())
    }
}

