use std::{path::{PathBuf}};

pub use cargo_metadata;

pub fn locate_bootloader() -> Result<BootloaderInfo, Error> {
    let project_metadata = cargo_metadata::MetadataCommand::new().exec()?;

    let kernel_manifest_path = locate_cargo_manifest::locate_manifest()?;

    let kernel_pkg = project_metadata
        .packages
        .iter()
        .find(|p| p.manifest_path == kernel_manifest_path)
        .ok_or_else(|| Error::KernelPackageNotFound {
            manifest_path: kernel_manifest_path.to_owned(),
        })?;

    let bootloader_pkg = bootloader_package(&project_metadata, kernel_pkg)?;

    let resolve_opt = project_metadata.resolve.as_ref();
    let resolve = resolve_opt.ok_or(Error::CargoMetadataIncomplete {
        key: "resolve".into(),
    })?;
    let bootloader_resolve = resolve
        .nodes
        .iter()
        .find(|n| n.id == bootloader_pkg.id)
        .ok_or(Error::CargoMetadataIncomplete {
            key: format!("resolve[\"{}\"]", bootloader_pkg.name),
        })?;
    let features = bootloader_resolve.features.clone();

    Ok(BootloaderInfo {
        package: bootloader_pkg.to_owned(),
        features,
        kernel_manifest_path,
    })
}

#[derive(Debug)]
pub struct BootloaderInfo {
    pub package: cargo_metadata::Package,
    pub features: Vec<String>,
    pub kernel_manifest_path: PathBuf,
}

/// There is something wrong with the bootloader dependency.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error occured while running `cargo_metadata`
    #[error("An error occured while running `cargo_metadata`")]
    CargoMetadata {
        #[from]
        metadata_error: cargo_metadata::Error,
    },
    
    /// Failed to locate cargo manifest
    #[error("Failed to locate the cargo manifest (`Cargo.toml`)")]
    LocateManifest(#[from] locate_cargo_manifest::LocateManifestError),
    
    /// Bootloader dependency not found
    #[error(
        "Bootloader dependency not found\n\n\
        You need to add a dependency on a crate named `bootloader` in your Cargo.toml."
    )]
    BootloaderNotFound,

    /// Could not find kernel package in cargo metadata
    #[error(
        "Could not find package with manifest path `{manifest_path}` in cargo metadata output"
    )]
    KernelPackageNotFound {
        /// The manifest path of the kernel package
        manifest_path: PathBuf,
    },
    
    /// Could not find some required information in the `cargo metadata` output
    #[error("Could not find required key `{key}` in cargo metadata output")]
    CargoMetadataIncomplete {
        /// The required key that was not found
        key: String,
    },
}

    /// Returns the package metadata for the bootloader crate
    fn bootloader_package<'a>(
        project_metadata: &'a cargo_metadata::Metadata,
        kernel_package: &cargo_metadata::Package,
    ) -> Result<&'a cargo_metadata::Package, Error> {
        let bootloader_name = {
            let mut dependencies = kernel_package.dependencies.iter();
            let bootloader_package = dependencies
                .find(|p| p.rename.as_ref().unwrap_or(&p.name) == "bootloader")
                .ok_or(Error::BootloaderNotFound)?;
            bootloader_package.name.clone()
        };
    
        project_metadata
            .packages
            .iter()
            .find(|p| p.name == bootloader_name)
            .ok_or(Error::CargoMetadataIncomplete {
                key: format!("packages[name = `{}`", &bootloader_name),
            })
    }