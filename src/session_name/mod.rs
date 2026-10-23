//! Session-name rules shared by explicit and derived session names.

pub mod derive_error;
pub mod derived_session_name;
pub mod logical_folder;

/// Check `value` against the rules for a tmux session name.
///
/// Returns a short, user-facing reason when the name is not valid. Both an explicit
/// `SESSION_NAME` and a name derived from the current folder use this one rule.
pub fn check(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        return Err("session name must not be empty");
    }

    if value.chars().any(char::is_control) {
        return Err("session name must not contain control characters");
    }

    // tmux reserves `:` and `.` to separate a session from a window or pane index in a target
    // specifier, so a session name containing either could be misinterpreted by tmux itself.
    if value.contains([':', '.']) {
        return Err("session name must not contain ':' or '.'");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn given_a_plain_name_when_checked_then_it_is_accepted() {
        assert!(check("alpha").is_ok());
    }

    #[test]
    fn given_a_blank_name_when_checked_then_it_is_rejected() {
        assert!(check("   ").is_err());
    }

    #[test]
    fn given_a_name_with_a_dot_when_checked_then_it_is_rejected() {
        assert!(check("my.project").is_err());
    }
}
