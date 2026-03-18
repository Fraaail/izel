pub mod manifest;
pub mod resolve;

pub use manifest::{Manifest, PackageInfo, Dependency, parse_manifest};
pub use resolve::{resolve_dependencies, fetch_package};
