//! Clipboard provider boundary for `ws clip`.

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::process::{Command, Stdio};

use thiserror::Error;

/// Environment variable overriding the configured clipboard provider command.
///
/// The value is a program followed by space-separated arguments, for example `wl-copy` or
/// `pbcopy`, using the same convention as `WS_TERMINAL_LAUNCHER`. `ws` writes the resolved value
/// to the provider's standard input and never shell-interprets the configured value. When unset,
/// `ws` uses `xclip -selection clipboard`.
pub const CLIPBOARD_ENV_VAR: &str = "WS_CLIPBOARD_PROVIDER";

/// Errors from sending a value to the clipboard provider.
#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("could not start the clipboard provider: {source}")]
    Invocation {
        #[source]
        source: io::Error,
    },
    #[error("the clipboard provider did not accept the value")]
    Failed,
}

/// Boundary for sending one value to the user's clipboard.
pub trait ClipboardProvider: Send + Sync {
    /// Send `value` to the clipboard without ever writing it to stdout, stderr, or a log.
    fn copy(&self, value: &str) -> Result<(), ClipboardError>;
}

/// Function signature used to spawn the clipboard provider and feed it `value` on stdin, kept
/// injectable so tests can observe the invocation without a real clipboard.
pub type ClipboardRunner = fn(&OsStr, &[OsString], &str) -> io::Result<bool>;

/// Sends values to a configured or default external clipboard provider command.
pub struct ConfiguredClipboardProvider {
    program: OsString,
    args: Vec<OsString>,
    runner: ClipboardRunner,
}

impl ConfiguredClipboardProvider {
    /// Build a provider from [`CLIPBOARD_ENV_VAR`], falling back to `xclip -selection clipboard`.
    pub fn from_env() -> Self {
        let (program, args) = env::var_os(CLIPBOARD_ENV_VAR)
            .and_then(|value| parse_command(&value))
            .unwrap_or_else(default_command);
        Self {
            program,
            args,
            runner: run_and_feed_stdin,
        }
    }

    /// Construct a provider with an explicit command and an injected runner, for tests.
    #[cfg(test)]
    pub(crate) fn with_command(
        program: impl Into<OsString>,
        args: Vec<OsString>,
        runner: ClipboardRunner,
    ) -> Self {
        Self {
            program: program.into(),
            args,
            runner,
        }
    }
}

impl ClipboardProvider for ConfiguredClipboardProvider {
    fn copy(&self, value: &str) -> Result<(), ClipboardError> {
        let accepted = (self.runner)(&self.program, &self.args, value)
            .map_err(|source| ClipboardError::Invocation { source })?;
        if accepted {
            Ok(())
        } else {
            Err(ClipboardError::Failed)
        }
    }
}

fn default_command() -> (OsString, Vec<OsString>) {
    (
        OsString::from("xclip"),
        vec![OsString::from("-selection"), OsString::from("clipboard")],
    )
}

fn parse_command(value: &OsStr) -> Option<(OsString, Vec<OsString>)> {
    let value = value.to_str()?;
    let mut parts = value.split_whitespace();
    let program = OsString::from(parts.next()?);
    let args = parts.map(OsString::from).collect();
    Some((program, args))
}

/// Spawn the provider and write `value` to its stdin, waiting for it to exit.
///
/// A provider that exits before reading its stdin (refusing the value) turns the write into a
/// broken-pipe I/O error; that error is deliberately swallowed here so the provider's own exit
/// status — checked below — stays the single source of truth for acceptance.
fn run_and_feed_stdin(program: &OsStr, args: &[OsString], value: &str) -> io::Result<bool> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(value.as_bytes());
    }
    Ok(child.wait()?.success())
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;

    use super::{ClipboardError, ClipboardProvider, ConfiguredClipboardProvider};

    fn asserting_runner(program: &OsStr, args: &[OsString], value: &str) -> io::Result<bool> {
        assert_eq!(program, OsStr::new("wl-copy"));
        assert!(args.is_empty());
        assert_eq!(value, "the-secret-value");
        Ok(true)
    }

    fn failing_runner(_: &OsStr, _: &[OsString], _: &str) -> io::Result<bool> {
        Ok(false)
    }

    #[test]
    fn given_a_configured_provider_when_copied_then_the_exact_value_is_sent_on_stdin() {
        let provider = ConfiguredClipboardProvider::with_command(
            "wl-copy",
            Vec::new(),
            asserting_runner as super::ClipboardRunner,
        );

        provider
            .copy("the-secret-value")
            .expect("a provider that accepts the value should succeed");
    }

    #[test]
    fn given_a_provider_that_refuses_the_value_when_copied_then_it_reports_failed() {
        let provider = ConfiguredClipboardProvider::with_command(
            "wl-copy",
            Vec::new(),
            failing_runner as super::ClipboardRunner,
        );

        let error = provider
            .copy("the-secret-value")
            .expect_err("a refusing provider should report failure");

        assert!(matches!(error, ClipboardError::Failed));
    }
}
