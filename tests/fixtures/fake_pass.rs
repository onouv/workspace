//! A minimal stand-in for `pass`, used by process-level integration tests to serve seeded
//! credential entries to `ws clip` (and to prove when `pass` was *not* contacted, such as during
//! `ws up`) without a real password store or GPG agent.
//!
//! `pass show NAME` looks up a file named after `NAME` (with `/` replaced by `_`) under the
//! directory named by `WS_FAKE_PASS_STORE`, printing its contents on success and exiting non-zero
//! when the entry is missing. Every invocation is also appended as one line to the file named by
//! `WS_FAKE_PASS_LOG`, when set, so tests can assert that `pass` was contacted zero or exactly
//! one time.

use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Ok(log_path) = env::var("WS_FAKE_PASS_LOG") {
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("the fake pass log file should be writable");
        writeln!(log, "{}", args.join(" ")).expect("the pass invocation should be recorded");
    }

    let store = env::var("WS_FAKE_PASS_STORE").expect("WS_FAKE_PASS_STORE must be set by the test");
    let name = args.get(1).expect("fake pass expects `show NAME`");
    let entry_path = Path::new(&store).join(name.replace('/', "_"));

    match fs::read_to_string(entry_path) {
        Ok(value) => print!("{value}"),
        Err(_) => std::process::exit(1),
    }
}
