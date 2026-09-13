//! A minimal stand-in for a separate-terminal launcher, used by process-level integration tests
//! to observe when and with what arguments `ws` invokes the configured launcher command.
//!
//! Every invocation's arguments are appended as one line to the file named by the
//! `WS_FAKE_LAUNCHER_LOG` environment variable, which the test harness must set.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    let log_path =
        env::var("WS_FAKE_LAUNCHER_LOG").expect("WS_FAKE_LAUNCHER_LOG must be set by the test");
    let args: Vec<String> = env::args().skip(1).collect();

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("the fake launcher log file should be writable");
    writeln!(log, "{}", args.join(" ")).expect("the launcher invocation should be recorded");
}
