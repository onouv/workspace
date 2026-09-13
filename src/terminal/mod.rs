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
}

// Consumed by the `down`/`exit` TTY-confirmation behavior added in a later phase (User Story 7).
#[allow(dead_code)]
impl TerminalContext {
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

#[cfg(test)]
impl TerminalContext {
    /// A context outside tmux with an interactive terminal on both streams.
    pub(crate) const fn outside_tmux_for_test() -> Self {
        Self {
            inside_tmux: false,
            stdin_is_tty: true,
            stdout_is_tty: true,
        }
    }

    /// A context inside a tmux client with an interactive terminal on both streams.
    pub(crate) const fn inside_tmux_for_test() -> Self {
        Self {
            inside_tmux: true,
            stdin_is_tty: true,
            stdout_is_tty: true,
        }
    }
}
