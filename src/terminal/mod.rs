#![allow(dead_code)] // Terminal capabilities are consumed by lifecycle tasks after this boundary.

use std::env;
use std::io::{self, IsTerminal};

pub mod launcher;

/// Terminal capabilities relevant to prompting and workspace attachment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalContext {
    inside_tmux: bool,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
}

impl TerminalContext {
    /// Detect the current process terminal capabilities.
    pub fn detect() -> Self {
        Self {
            inside_tmux: env::var_os("TMUX").is_some(),
            stdin_is_tty: io::stdin().is_terminal(),
            stdout_is_tty: io::stdout().is_terminal(),
        }
    }

    /// Return whether the process is running inside a tmux client.
    pub const fn inside_tmux(self) -> bool {
        self.inside_tmux
    }

    /// Return whether interactive input is available.
    pub const fn stdin_is_tty(self) -> bool {
        self.stdin_is_tty
    }

    /// Return whether interactive output is available.
    pub const fn stdout_is_tty(self) -> bool {
        self.stdout_is_tty
    }

    /// Return whether prompting is safe for this process.
    pub const fn is_interactive(self) -> bool {
        self.stdin_is_tty && self.stdout_is_tty
    }
}

impl Default for TerminalContext {
    fn default() -> Self {
        Self::detect()
    }
}
