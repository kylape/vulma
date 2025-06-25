use std::{error::Error, fmt::Display, str::FromStr};

use storage::EmbeddedImageScanComponent;

pub mod storage {
    tonic::include_proto!("storage");
}

pub mod virtual_machine_service_client {
    tonic::include_proto!("sensor");
}

#[derive(Debug, PartialEq)]
pub enum ClientError {
    ParseError(String),
    MissingPackageName,
    MissingVersionName(String),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::ParseError(e) => write!(f, "Rpm parsing failed: {e}"),
            ClientError::MissingPackageName => write!(f, "Missing package name"),
            ClientError::MissingVersionName(pkg) => write!(f, "Missing version for '{pkg}'"),
        }
    }
}

impl Error for ClientError {}

impl storage::EmbeddedImageScanComponent {
    fn new(name: &str, version: &str, release: &str, arch: &str) -> Self {
        let mut s = EmbeddedImageScanComponent::default();
        s.name = name.into();
        s.version = format!("{version}-{release}");
        s.architecture = arch.into();

        s
    }
}

impl FromStr for storage::EmbeddedImageScanComponent {
    type Err = ClientError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let fields = value.split('|').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(ClientError::ParseError(format!(
                "Invalid field count: expected=4, got={}",
                fields.len()
            )));
        }

        let name = fields[0];
        let version = fields[1];
        let release = fields[2];
        let arch = fields[3];

        // Sanity check the package information
        if name.is_empty() {
            Err(ClientError::MissingPackageName)
        } else if version.is_empty() {
            Err(ClientError::MissingVersionName(name.to_string()))
        } else {
            Ok(EmbeddedImageScanComponent::new(
                name, version, release, arch,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        let tests = [
            (
                "kernel|6.14.9|300.fc42|x86_64",
                Ok(EmbeddedImageScanComponent::new(
                    "kernel", "6.14.9", "300.fc42", "x86_64",
                )),
            ),
            (
                "kernel|6.14.9|300.fc42",
                Err(ClientError::ParseError(
                    "Invalid field count: expected=4, got=3".to_string(),
                )),
            ),
            (
                "kernel|6.14.9|300.fc42|x86_64|",
                Err(ClientError::ParseError(
                    "Invalid field count: expected=4, got=5".to_string(),
                )),
            ),
            (
                "|6.14.9|300.fc42|x86_64",
                Err(ClientError::MissingPackageName),
            ),
            (
                "kernel||300.fc42|x86_64",
                Err(ClientError::MissingVersionName("kernel".to_string())),
            ),
        ];

        for (input, expected) in tests {
            assert_eq!(str::parse(input), expected);
        }
    }
}
