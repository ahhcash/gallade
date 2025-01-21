use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Eq, Hash)]
pub struct MavenVersion {
    major: u32,
    minor: u32,
    patch: u32,
    qualifier: Option<String>,
}

#[derive(Debug)]
pub enum VersionParseError {
    InvalidFormat,
    InvalidNumber(std::num::ParseIntError),
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid version format"),
            Self::InvalidNumber(e) => write!(f, "invalid number in version: {}", e),
        }
    }
}

impl std::error::Error for VersionParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidFormat => None,
            Self::InvalidNumber(e) => Some(e),
        }
    }
}

impl FromStr for MavenVersion {
    type Err = VersionParseError;

    fn from_str(version: &str) -> Result<Self, Self::Err> {
        // Split version and qualifier
        let (version_part, qualifier) = match version.split_once('-') {
            Some((v, q)) => (v, Some(q.to_string())),
            None => (version, None),
        };

        // Parse numeric components
        let nums: Vec<&str> = version_part.split('.').collect();

        match nums.len() {
            3 => Ok(Self {
                major: nums[0].parse().map_err(VersionParseError::InvalidNumber)?,
                minor: nums[1].parse().map_err(VersionParseError::InvalidNumber)?,
                patch: nums[2].parse().map_err(VersionParseError::InvalidNumber)?,
                qualifier,
            }),
            2 => Ok(Self {
                major: nums[0].parse().map_err(VersionParseError::InvalidNumber)?,
                minor: nums[1].parse().map_err(VersionParseError::InvalidNumber)?,
                patch: 0,
                qualifier,
            }),
            1 => Ok(Self {
                major: nums[0].parse().map_err(VersionParseError::InvalidNumber)?,
                minor: 0,
                patch: 0,
                qualifier,
            }),
            _ => Err(VersionParseError::InvalidFormat),
        }
    }
}

impl PartialEq for MavenVersion {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.qualifier == other.qualifier
    }
}

impl PartialOrd for MavenVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for MavenVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.qualifier {
            Some(q) => write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, q),
            None => write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl Ord for MavenVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {},
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {},
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {},
            ord => return ord,
        }

        match (&self.qualifier, &other.qualifier) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum VersionReq {
    Exact(MavenVersion),
    Range {
        min: Option<MavenVersion>,
        min_inclusive: bool,
        max: Option<MavenVersion>,
        max_inclusive: bool,
    },
    /// Special version requirements
    Latest,
    Release,
}

impl VersionReq {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        // Handle special versions first
        match input.trim().to_uppercase().as_str() {
            "LATEST" => return Ok(Self::Latest),
            "RELEASE" => return Ok(Self::Release),
            _ => {}
        }

        // Check if it's a range expression
        if input.starts_with('[') || input.starts_with('(') {
            if !input.ends_with(']') && !input.ends_with(')') {
                anyhow::bail!("invalid range format: missing closing bracket");
            }

            let min_inclusive = input.starts_with('[');
            let max_inclusive = input.ends_with(']');

            // Remove brackets and split on comma
            let content = &input[1..input.len()-1];
            let parts: Vec<&str> = content.split(',').collect();

            if parts.len() != 2 {
                anyhow::bail!("invalid range format: expected two versions separated by comma");
            }

            let min = if parts[0].trim().is_empty() {
                None
            } else {
                Some(parts[0].trim().parse()?)
            };

            let max = if parts[1].trim().is_empty() {
                None
            } else {
                Some(parts[1].trim().parse()?)
            };

            return Ok(Self::Range {
                min,
                min_inclusive,
                max,
                max_inclusive,
            });
        }

        // If not a range or special version, treat as exact version
        Ok(Self::Exact(input.parse()?))
    }

    pub fn matches(&self, version: &MavenVersion) -> bool {
        match self {
            Self::Exact(req) => req == version,
            Self::Range { min, min_inclusive, max, max_inclusive } => {
                // Check minimum bound
                let meets_min = match (min, min_inclusive) {
                    (None, _) => true,
                    (Some(min), true) => version >= min,
                    (Some(min), false) => version > min,
                };

                let meets_max = match (max, max_inclusive) {
                    (None, _) => true,
                    (Some(max), true) => version <= max,
                    (Some(max), false) => version < max,
                };

                meets_min && meets_max
            }
            // For Latest and Release, we'll handle these specially when resolving dependencies
            Self::Latest | Self::Release => true,
        }
    }
}

impl FromStr for VersionReq {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert!("32.1.3-jre".parse::<MavenVersion>().is_ok());
        assert!("1.2.3".parse::<MavenVersion>().is_ok());
        assert!("1.2".parse::<MavenVersion>().is_ok());
        assert!("1".parse::<MavenVersion>().is_ok());
        assert!("abc".parse::<MavenVersion>().is_err());
    }

    #[test]
    fn test_version_req_parsing() {
        let req = VersionReq::parse("1.2.3").unwrap();
        assert!(matches!(req, VersionReq::Exact(_)));

        let req = VersionReq::parse("[1.2.0,2.0.0)").unwrap();
        match req {
            VersionReq::Range { min_inclusive, max_inclusive, .. } => {
                assert!(min_inclusive);
                assert!(!max_inclusive);
            }
            _ => panic!("expected range"),
        }

        assert!(matches!(VersionReq::parse("LATEST").unwrap(), VersionReq::Latest));
        assert!(matches!(VersionReq::parse("RELEASE").unwrap(), VersionReq::Release));
    }

    #[test]
    fn test_version_req_matching() {
        let v1: MavenVersion = "1.2.3".parse().unwrap();
        let v2: MavenVersion = "1.5.0".parse().unwrap();
        let v3: MavenVersion = "2.0.0".parse().unwrap();

        // Test exact version matching
        let req = VersionReq::parse("1.2.3").unwrap();
        assert!(req.matches(&v1));
        assert!(!req.matches(&v2));

        let req = VersionReq::parse("[1.2.0,2.0.0)").unwrap();
        assert!(req.matches(&v1));
        assert!(req.matches(&v2));
        assert!(!req.matches(&v3));

        let req = VersionReq::parse("(1.2.0,2.0.0)").unwrap();
        assert!(req.matches(&v2));
        assert!(!req.matches(&v3));
    }

    #[test]
    fn test_version_comparison() {
        let v1: MavenVersion = "1.2.3".parse().unwrap();
        let v2: MavenVersion = "1.2.4".parse().unwrap();
        let v3: MavenVersion = "1.2.3-jre".parse().unwrap();

        assert!(v1 < v2);
        assert!(v1 > v3);
        assert!(v2 > v3);
    }
}