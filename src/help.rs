//! Side-effect-free help and reference rendering.

use clap::CommandFactory;

use crate::cli::Cli;

/// Render the top-level command summary using clap's canonical usage and command list.
///
/// This function only formats static command metadata. It must not inspect the current project,
/// invoke an external process, prompt, or access the credential store.
pub fn general() -> String {
    Cli::command().render_help().to_string()
}

/// Return the normative `.ws` YAML language reference compiled into the executable.
///
/// Embedding the reference keeps `ws help config` available when the current project has no `.ws`
/// file and avoids a runtime dependency on the repository or working directory.
pub fn config() -> &'static str {
    include_str!("../doc/config-lang.md")
}

/// Return the normative `ws clip` credential-mapping YAML language reference compiled into the
/// executable.
///
/// Embedding the reference keeps `ws help clip` available without reading the user- or
/// project-level mapping file, contacting `pass`, or contacting the clipboard provider.
pub fn clip() -> &'static str {
    include_str!("../doc/clip-config-lang.md")
}
