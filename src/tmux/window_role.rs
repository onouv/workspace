//! The role of one window in a launch plan.

use std::path::Path;

/// Whether a window creates the session or joins it.
#[derive(Clone, Copy, Debug)]
pub enum WindowRole<'a> {
    /// The first window. It creates the session and records `project_folder` in the session
    /// option [`super::session::PROJECT_FOLDER_OPTION`].
    First { project_folder: &'a Path },
    /// A later window in an existing session.
    Additional,
}
