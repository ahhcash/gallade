#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MavenVersion {
    major: u32,
    minor: u32,
    patch: u32,
    qualifier: Option<String>,
}

impl MavenVersion {
    pub fn parse(version: &str) -> anyhow::Result<Self> {
        // Split on hyphen first to separate qualifier
        let (version_part, qualifier) = match version.split_once('-') {
            Some((v, q)) => (v, Some(q.to_string())),
            None => (version, None),
        };

        // Parse the numeric parts
        let nums: Vec<&str> = version_part.split('.').collect();
        match nums.len() {
            3 => Ok(Self {
                major: nums[0].parse()?,
                minor: nums[1].parse()?,
                patch: nums[2].parse()?,
                qualifier,
            }),
            2 => Ok(Self {
                major: nums[0].parse()?,
                minor: nums[1].parse()?,
                patch: 0,
                qualifier,
            }),
            1 => Ok(Self {
                major: nums[0].parse()?,
                minor: 0,
                patch: 0,
                qualifier,
            }),
            _ => anyhow::bail!("invalid version format"),
        }
    }
}

#[derive(Debug)]
pub enum VersionReq {
    Exact(MavenVersion),
    Range {
        min: Option<MavenVersion>,
        max: Option<MavenVersion>,
    },
    Any,
}