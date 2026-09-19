//! On-demand password-store provider boundary.
//!
//! `ws` never receives, caches, or exports the password-store master passphrase. `ws clip`
//! requests exactly one named entry through the user's existing `pass`/GPG-agent unlock flow and
//! streams it directly to its approved destination (see `src/lifecycle/clip.rs`).

use std::ffi::{OsStr, OsString};
use std::io;
use std::process::{Command, Output};

use thiserror::Error;

/// A password-store value that must never be displayed, logged, or persisted.
///
/// Deliberately opaque: `Debug` and `Display` are not implemented, so an accidental `{:?}` or
/// `{}` in a future log statement or error message fails to compile instead of leaking it.
pub struct Secret(String);

impl Secret {
    /// Borrow the value for the one approved destination that consumes it (for example, a
    /// clipboard provider's stdin) — never for display, logging, or persistence.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Errors from requesting a password-store entry.
///
/// Unlike tmux's own diagnostics, `pass`/GPG-agent stderr may itself contain sensitive detail
/// (key ids, agent socket paths, prior attempt context), so it is deliberately not included here.
#[derive(Debug, Error)]
pub enum CredentialError {
    /// `pass` could not be started or contacted at all.
    #[error("could not start or contact pass: {source}")]
    Invocation {
        #[source]
        source: io::Error,
    },
    /// `pass` ran but could not provide the requested entry (locked, missing, unauthorized).
    #[error("pass could not provide the requested entry")]
    Unavailable,
}

/// Function signature used to execute an external command and capture its output.
///
/// Keeping command execution injectable lets tests verify the requested entry name without a
/// real password store.
pub type CommandRunner = fn(&OsStr, &[OsString]) -> io::Result<Output>;

/// Requests one named entry from the `pass` password store, on demand.
pub struct PassProvider {
    runner: CommandRunner,
}

impl PassProvider {
    /// Construct a provider that invokes the real `pass` executable.
    pub fn new() -> Self {
        Self::with_runner(run_system_command)
    }

    /// Construct a provider with an injected command runner, for tests.
    pub const fn with_runner(runner: CommandRunner) -> Self {
        Self { runner }
    }

    /// Request the entry named `name` via `pass show`, relying entirely on the user's existing
    /// GPG-agent cache to unlock it; this call never receives or caches a master passphrase.
    pub fn reveal(&self, name: &str) -> Result<Secret, CredentialError> {
        let output = (self.runner)(
            OsStr::new("pass"),
            &[OsString::from("show"), OsString::from(name)],
        )
        .map_err(|source| CredentialError::Invocation { source })?;

        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout)
                .trim_end_matches('\n')
                .to_owned();
            Ok(Secret(value))
        } else {
            Err(CredentialError::Unavailable)
        }
    }
}

impl Default for PassProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn run_system_command(program: &OsStr, args: &[OsString]) -> io::Result<Output> {
    Command::new(program).args(args).output()
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};

    use super::{CommandRunner, CredentialError, PassProvider};

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    #[test]
    fn given_a_requested_entry_when_revealed_then_only_that_entry_is_requested() {
        fn asserting_runner(program: &OsStr, args: &[OsString]) -> io::Result<Output> {
            assert_eq!(program, OsStr::new("pass"));
            assert_eq!(args, [OsString::from("show"), OsString::from("git/ssh")]);
            Ok(Output {
                status: success_status(),
                stdout: b"the-secret-value\n".to_vec(),
                stderr: Vec::new(),
            })
        }
        let provider = PassProvider::with_runner(asserting_runner as CommandRunner);

        let secret = provider
            .reveal("git/ssh")
            .expect("the fake entry should resolve");

        assert_eq!(secret.expose(), "the-secret-value");
    }

    #[test]
    fn given_pass_is_locked_when_revealed_then_no_detail_is_included_in_the_error() {
        // Scenario: US8-AS3
        fn locked_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
            Ok(Output {
                status: failure_status(),
                stdout: Vec::new(),
                stderr: b"gpg: decryption failed: No secret key".to_vec(),
            })
        }
        let provider = PassProvider::with_runner(locked_runner as CommandRunner);

        // `Secret` deliberately does not implement `Debug`, so `expect_err` (which would need
        // to format the success case) is not usable here; match explicitly instead.
        let error = match provider.reveal("git/ssh") {
            Ok(_) => panic!("a locked store should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, CredentialError::Unavailable));
        assert!(!error.to_string().contains("secret key"));
    }
}
