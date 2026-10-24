//! One-line user notices on stderr, kept behind a trait so lifecycle code stays testable.

/// Destination for one-line notices that are not part of a command's stdout result.
pub trait Notifier {
    /// Show one notice line to the user.
    fn notify(&self, line: &str);
}

/// Writes each notice as one `ws: ` prefixed line to stderr.
pub struct StderrNotifier;

impl Notifier for StderrNotifier {
    fn notify(&self, line: &str) {
        eprintln!("ws: {line}");
    }
}

/// Records notices in memory so unit tests can assert on them.
#[cfg(test)]
#[derive(Default)]
pub struct RecordingNotifier {
    lines: std::cell::RefCell<Vec<String>>,
}

#[cfg(test)]
impl RecordingNotifier {
    /// The notices recorded so far, in order.
    pub fn lines(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }
}

#[cfg(test)]
impl Notifier for RecordingNotifier {
    fn notify(&self, line: &str) {
        self.lines.borrow_mut().push(line.to_owned());
    }
}
