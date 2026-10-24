//! Where `ws up` takes the workspace definition from after the setup-file step.

use std::path::PathBuf;

/// The source of the workspace definition for a new session (002 FR-004, FR-009).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetupSource {
    /// Load this `.ws` file through the normal loader.
    File(PathBuf),
    /// `ws` could not write `.ws`: parse the default content from memory. `source_path` is the
    /// `.ws` path, so `path: .` still resolves to the project folder.
    InMemoryDefault { source_path: PathBuf },
}
