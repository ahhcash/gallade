use std::process::Command;
use crate::classpath::{ClasspathBuilder, JavaEnvironment};
use crate::coordinates::Coordinate;
use crate::manifest::Manifest;
use crate::repository::Repository;

pub struct BuildOptions {
    pub args: Vec<String>,
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

        let java_env = JavaEnvironment::new(classpath)?;

        let main_class = self.manifest.project.main_class
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no main class specified in manifest"))?;

        let mut cmd = Command::new("java");

        if options.debug {
            cmd.arg("-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005");
        }

        cmd.arg("-cp")
            .arg(&java_env.classpath)
            .arg(main_class);

        cmd.args(&options.args);

        let status = cmd.status()?;

        if !status.success() {
            anyhow::bail!("java process failed with status: {}", status);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_builder_setup() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let repo = Repository::new(temp.path().to_path_buf());

        let manifest_content = r#"
            [project]
            name = "test-project"
            version = "0.1.0"
            main_class = "com.example.Main"

            [dependencies]
            "org.slf4j:slf4j-api" = "1.7.36"
        "#;

        let manifest: Manifest = toml::from_str(manifest_content)?;
        let builder = Builder::new(manifest, repo);

        assert_eq!(builder.manifest.project.name, "test-project");
        Ok(())
    }
}