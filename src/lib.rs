//! Helper crate for locating a `bootloader` dependency on the file system.

#![warn(missing_docs)]

use std::{convert, fmt, io, path::PathBuf, process::Command, string};

/// Locates the dependency with the given name on the file system.
///
/// Returns the manifest path of the bootloader, i.e. the path to the Cargo.toml on the file
/// system.
pub fn locate_bootloader(dependency_name: &str, path: Option<PathBuf>) -> Result<PathBuf, LocateError> {
    let metadata = metadata(path)?;

    let root = metadata["resolve"]["root"]
        .as_str()
        .ok_or(LocateError::MetadataInvalid)?;

    let root_resolve = metadata["resolve"]["nodes"]
        .members()
        .find(|r| r["id"] == root)
        .ok_or(LocateError::MetadataInvalid)?;

    let dependency = root_resolve["deps"]
        .members()
        .find(|d| d["name"] == dependency_name)
        .ok_or(LocateError::DependencyNotFound)?;
    let dependency_id = dependency["pkg"]
        .as_str()
        .ok_or(LocateError::MetadataInvalid)?;

    let dependency_package = metadata["packages"]
        .members()
        .find(|p| p["id"] == dependency_id)
        .ok_or(LocateError::MetadataInvalid)?;
    let dependency_manifest = dependency_package["manifest_path"]
        .as_str()
        .ok_or(LocateError::MetadataInvalid)?;

    Ok(dependency_manifest.into())
}

/// Failed to locate the bootloader dependency with the given name.
#[derive(Debug)]
pub enum LocateError {
    /// The project metadata returned from `cargo metadata` was not valid.
    MetadataInvalid,
    /// No dependency with the given name found in the project metadata.
    DependencyNotFound,
    /// Failed to query project metadata.
    Metadata(CargoMetadataError),
}

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocateError::MetadataInvalid => write!(f, "The `cargo metadata` output was not valid"),
            LocateError::DependencyNotFound => write!(
                f,
                "Could not find a dependency with the given name in the `cargo metadata` output"
            ),
            LocateError::Metadata(source) => {
                write!(f, "Failed to retrieve project metadata: {}", source)
            }
        }
    }
}

impl std::error::Error for LocateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocateError::MetadataInvalid => None,
            LocateError::DependencyNotFound => None,
            LocateError::Metadata(source) => Some(source),
        }
    }
}

impl convert::From<CargoMetadataError> for LocateError {
    fn from(source: CargoMetadataError) -> Self {
        LocateError::Metadata(source)
    }
}

fn metadata(path: Option<PathBuf>) -> Result<json::JsonValue, CargoMetadataError> {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("metadata");
    cmd.arg("--manifest-path").arg(path.unwrap_or(PathBuf::from("./Cargo.toml")));
    cmd.arg("--format-version").arg("1");
    let output = cmd.output()?;

    if !output.status.success() {
        return Err(CargoMetadataError::Failed {
            stderr: output.stderr,
        });
    }

    let output = String::from_utf8(output.stdout)?;
    let parsed = json::parse(&output)?;

    Ok(parsed)
}
/// Failed to query project metadata.
#[derive(Debug)]
pub enum CargoMetadataError {
    /// An I/O error that occurred while trying to execute `cargo metadata`.
    Io(io::Error),
    /// The command `cargo metadata` did not exit successfully.
    Failed {
        /// The standard error output of `cargo metadata`.
        stderr: Vec<u8>,
    },
    /// The output of `cargo metadata` was not valid UTF-8.
    StringConversion(string::FromUtf8Error),
    /// An error occurred while parsing the output of `cargo metadata` as JSON.
    ParseJson(json::Error),
}

impl fmt::Display for CargoMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CargoMetadataError::Io(err) => write!(f, "Failed to execute `cargo metadata`: {}", err),
            CargoMetadataError::Failed { stderr } => write!(
                f,
                "`cargo metadata` was not successful: {}",
                String::from_utf8_lossy(stderr)
            ),
            CargoMetadataError::StringConversion(err) => write!(
                f,
                "Failed to convert the `cargo metadata` output to a string: {}",
                err
            ),
            CargoMetadataError::ParseJson(err) => write!(
                f,
                "Failed to parse `cargo metadata` output as JSON: {}",
                err
            ),
        }
    }
}

impl std::error::Error for CargoMetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CargoMetadataError::Io(err) => Some(err),
            CargoMetadataError::Failed { stderr: _ } => None,
            CargoMetadataError::StringConversion(err) => Some(err),
            CargoMetadataError::ParseJson(err) => Some(err),
        }
    }
}

impl convert::From<io::Error> for CargoMetadataError {
    fn from(source: io::Error) -> Self {
        CargoMetadataError::Io(source)
    }
}

impl convert::From<string::FromUtf8Error> for CargoMetadataError {
    fn from(source: string::FromUtf8Error) -> Self {
        CargoMetadataError::StringConversion(source)
    }
}

impl convert::From<json::Error> for CargoMetadataError {
    fn from(source: json::Error) -> Self {
        CargoMetadataError::ParseJson(source)
    }
}
