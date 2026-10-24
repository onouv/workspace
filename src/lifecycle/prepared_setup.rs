//! The result of the setup-file step: where the workspace comes from and what to tell the user.

use crate::lifecycle::setup_notice::SetupNotice;
use crate::lifecycle::setup_source::SetupSource;

/// The outcome of preparing `.ws` for a new session (002).
///
/// `up` loads the workspace from `source` and shows `notice`, if any, through the notifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedSetup {
    source: SetupSource,
    notice: Option<SetupNotice>,
}

impl PreparedSetup {
    /// Combine a workspace source with the line to show, if any.
    pub const fn new(source: SetupSource, notice: Option<SetupNotice>) -> Self {
        Self { source, notice }
    }

    /// Where the workspace definition comes from.
    pub const fn source(&self) -> &SetupSource {
        &self.source
    }

    /// The file message or warning to show, if any.
    pub const fn notice(&self) -> Option<&SetupNotice> {
        self.notice.as_ref()
    }
}
