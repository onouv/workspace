//! Where the session name of `ws up` comes from.

use crate::session_name::derived_session_name::DerivedSessionName;

/// The session name of one `ws up` call and its source.
#[derive(Clone, Debug)]
pub enum SessionNameSource {
    /// The user typed `SESSION_NAME`. Feature 001 rules apply without change.
    Explicit(String),
    /// The user gave no name; `ws` made it from the current folder (003 FR-001).
    Derived(DerivedSessionName),
}

impl SessionNameSource {
    /// The session name, whatever its source.
    pub fn name(&self) -> &str {
        match self {
            Self::Explicit(name) => name,
            Self::Derived(derived) => derived.name(),
        }
    }
}
