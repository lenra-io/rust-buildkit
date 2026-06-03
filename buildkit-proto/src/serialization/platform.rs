use std::fmt;
use std::str::FromStr;

use crate::pb::Platform;

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, '/').collect();
        Ok(Platform {
            os: parts.first().copied().unwrap_or("linux").to_string(),
            architecture: parts.get(1).copied().unwrap_or("amd64").to_string(),
            variant: parts.get(2).copied().unwrap_or("").to_string(),
            ..Platform::default()
        })
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.variant.is_empty() {
            write!(f, "{}/{}", self.os, self.architecture)
        } else {
            write!(f, "{}/{}/{}", self.os, self.architecture, self.variant)
        }
    }
}
