//! A minimal stand-in for a clipboard provider (`xclip`), used by process-level integration
//! tests to observe the value `ws clip` sends without a real clipboard.
//!
//! The full contents written to stdin are saved to the file named by `WS_FAKE_CLIPBOARD_LOG`. If
//! `WS_FAKE_CLIPBOARD_FAIL` is set, stdin is still read and saved, but the process exits
//! non-zero, simulating a provider that refuses the value.

use std::env;
use std::fs;
use std::io::Read;

fn main() {
    let log_path =
        env::var("WS_FAKE_CLIPBOARD_LOG").expect("WS_FAKE_CLIPBOARD_LOG must be set by the test");

    let mut value = String::new();
    std::io::stdin()
        .read_to_string(&mut value)
        .expect("the fake clipboard provider should be able to read stdin");
    fs::write(&log_path, &value).expect("the fake clipboard log file should be writable");

    if env::var_os("WS_FAKE_CLIPBOARD_FAIL").is_some() {
        std::process::exit(1);
    }
}
