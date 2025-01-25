use std::process::Command;
use walkdir::WalkDir;
use crate::classpath::{ClasspathBuilder, JavaEnvironment};
use crate::coordinates::Coordinate;
use crate::manifest::Manifest;
use crate::repository::Repository;

pub struct BuildOptions {
    pub debug: bool,
}

pub struct Builder {
    manifest: Manifest,
    repository: Repository,
}

impl Builder {
    pub fn new(manifest: Manifest, repository: Repository) -> Self {
        Self {
            manifest,
            repository,
        }
    }

    fn find_java_files(&self) -> anyhow::Result<Vec<String>> {
        let src_dir = std::env::current_dir()?.join("src").join("main").join("java");

        let java_files: Vec<String> = WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("java"))
            .map(|e| e.path().to_string_lossy().into_owned())
            .collect();

        Ok(java_files)
    }

    pub fn build(&self, options: BuildOptions) -> anyhow::Result<()> {
        let mut cp_builder = ClasspathBuilder::new(self.repository.clone());

        for (coord_str, dep_spec) in &self.manifest.deps {
            let coord = Coordinate::parse(coord_str)?;
            let version = match dep_spec {
                crate::manifest::DepSpec::Simple(v) => v,
                crate::manifest::DepSpec::Detailed { version, .. } => version,
            };
            cp_builder.with_dep(&coord, version);
        }

        let classpath = cp_builder.build();

        let java_files = self.find_java_files()?;
        if java_files.is_empty() {
            anyhow::bail!("no java source files found in src/main/java");
        }

        let target_dir = std::env::current_dir()?.join("target").join("classes");
        std::fs::create_dir_all(&target_dir)?;

        let mut javac = Command::new("javac");
        javac.args([
            "-d", target_dir.to_str().unwrap(),
            "-cp", &classpath
        ]);

        // Add debug info if requested
        if options.debug {
            javac.arg("-g");
        }

        javac.args(&java_files);

        let status = javac.status()?;
        if !status.success() {
            anyhow::bail!("compilation failed with status: {}", status);
        }

        Ok(())
    }
}