use std::{fmt::Display, process::Command};

#[derive(Debug, Default)]
struct Rpm {
    name: String,
    version: String,
    release: String,
    arch: String,
    sha256: String,
}

impl From<&str> for Rpm {
    fn from(value: &str) -> Self {
        let mut rpm = Rpm::default();
        for field in value.split('|').enumerate() {
            match field {
                (0, name) => rpm.name = name.to_string(),
                (1, version) => rpm.version = version.to_string(),
                (2, release) => rpm.release = release.to_string(),
                (3, arch) => rpm.arch = arch.to_string(),
                (4, sha256) => rpm.sha256 = sha256.to_string(),
                (i, v) => panic!("Unexpected element {i}: {v}"),
            }
        }
        rpm
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

fn main() {
    let rpm = Command::new("rpm")
        .args([
            "-qa",
            "--qf",
            "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{SHA256HEADER}\n",
        ])
        .output()
        .expect("rpm command failed");
    let stdout = str::from_utf8(rpm.stdout.as_slice()).expect("Failed to read stdout");

    for pkg in stdout.lines() {
        let rpm: Rpm = pkg.into();
        println!("{rpm}")
    }
}
