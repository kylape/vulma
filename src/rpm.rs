use std::{error::Error, fmt::Display, str::FromStr};

use serde::Serialize;

#[derive(Debug, PartialEq)]
pub enum RpmError {
    ParseError(String),
    MissingPackageName,
    MissingVersionName(String),
}

impl Display for RpmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpmError::ParseError(e) => write!(f, "Rpm parsing failed: {e}"),
            RpmError::MissingPackageName => write!(f, "Missing package name"),
            RpmError::MissingVersionName(pkg) => write!(f, "Missing version for '{pkg}'"),
        }
    }
}

impl Error for RpmError {}

#[derive(Debug, PartialEq, Serialize)]
pub struct Rpm {
    name: String,
    version: String,
    release: String,
    arch: String,
    sha256: String,
}

impl Rpm {
    fn new(name: &str, version: &str, release: &str, arch: &str, sha256: &str) -> Self {
        Rpm {
            name: name.to_string(),
            version: version.to_string(),
            release: release.to_string(),
            arch: arch.to_string(),
            sha256: sha256.to_string(),
        }
    }
}

impl FromStr for Rpm {
    type Err = RpmError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let fields = value.split('|').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(RpmError::ParseError(format!(
                "Invalid field count: expected=5, got={}",
                fields.len()
            )));
        }

        let name = fields[0];
        let version = fields[1];
        let release = fields[2];
        let arch = fields[3];
        let sha256 = fields[4];

        // Sanity check the package information
        if name.is_empty() {
            Err(RpmError::MissingPackageName)
        } else if version.is_empty() {
            Err(RpmError::MissingVersionName(name.to_string()))
        } else {
            Ok(Rpm::new(name, version, release, arch, sha256))
        }
    }
}

impl Display for Rpm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {} - {}",
            self.name, self.version, self.release, self.arch, self.sha256
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        let tests = [
            ("kernel|6.14.9|300.fc42|x86_64|83fe65b4a880e9201a5800430c57b52e55ebb0234c2430a481363059134c774b", Ok(Rpm::new( "kernel", "6.14.9", "300.fc42", "x86_64", "83fe65b4a880e9201a5800430c57b52e55ebb0234c2430a481363059134c774b"))),
            ("kernel|6.14.9|300.fc42|x86_64", Err(RpmError::ParseError("Invalid field count: expected=5, got=4".to_string()))),
            ("kernel|6.14.9|300.fc42|x86_64||", Err(RpmError::ParseError("Invalid field count: expected=5, got=6".to_string()))),
            ("|6.14.9|300.fc42|x86_64|83fe65b4a880e9201a5800430c57b52e55ebb0234c2430a481363059134c774b", Err(RpmError::MissingPackageName)),
            ("kernel||300.fc42|x86_64|83fe65b4a880e9201a5800430c57b52e55ebb0234c2430a481363059134c774b", Err(RpmError::MissingVersionName("kernel".to_string()))),
        ];

        for (input, expected) in tests {
            assert_eq!(str::parse(input), expected);
        }
    }
}
