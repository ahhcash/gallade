use std::fmt;


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Coordinate {
    pub namespace: String,
    pub name: String,
    pub version: Option<String>
}

impl Coordinate {
    pub fn parse(coord: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = coord.split(':').collect();
        match parts.len() {
            2 => Ok(Self {
                namespace: parts[0].to_string(),
                name: parts[1].to_string(),
                version: None,
            }),
            3 => Ok(Self {
                namespace: parts[0].to_string(),
                name: parts[1].to_string(),
                version: Some(parts[2].to_string()),
            }),
            _ => anyhow::bail!("invalid coordinate format - expected namespace:name[:version]")
        }
    }

    pub fn to_path(&self) -> String {
        format!(
            "{}/{}",
            self.namespace.replace('.', "/"),
            self.name
        )
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}:{}:{}", self.namespace, self.name, v),
            None => write!(f, "{}:{}", self.namespace, self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_version() {
        let coord = Coordinate::parse("com.google.guava:guava:31.1-jre").unwrap();
        assert_eq!(coord.namespace, "com.google.guava");
        assert_eq!(coord.name, "guava");
        assert_eq!(coord.version, Some("31.1-jre".to_string()));
    }

    #[test]
    fn test_parse_without_version() {
        let coord = Coordinate::parse("org.slf4j:slf4j-api").unwrap();
        assert_eq!(coord.namespace, "org.slf4j");
        assert_eq!(coord.name, "slf4j-api");
        assert_eq!(coord.version, None);
    }

    #[test]
    fn test_to_path() {
        let coord = Coordinate::parse("com.google.guava:guava").unwrap();
        assert_eq!(coord.to_path(), "com/google/guava/guava");
    }
}
