use clap::{Args, Parser, Subcommand, ValueEnum};

/// Top-level command-line model for `ws`.
#[derive(Debug, Parser)]
#[command(
    name = "ws",
    version,
    about = "Start and manage configured tmux workspaces",
    after_help = "Examples:\n  ws up SESSION_NAME\n  ws change SESSION_NAME\n  ws help config\n  ws help clip",
    disable_help_subcommand = true,
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Commands supported by the workspace CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Display general help or a specific reference.
    Help(HelpArgs),
    /// Create a workspace from `.ws`, or connect to an existing session.
    Up(UpArgs),
    /// Switch the current tmux client to an existing session.
    Change(SessionArgs),
    /// Terminate a workspace session after confirmation.
    Down(DownArgs),
    /// Detach the current tmux client without terminating its session.
    Exit,
    /// Send one configured credential to the clipboard.
    Clip(ClipArgs),
}

/// Arguments for the help command.
#[derive(Debug, Args)]
pub struct HelpArgs {
    /// Optional reference topic.
    #[arg(value_enum)]
    pub topic: Option<HelpTopic>,
}

/// Locally available help references.
#[derive(Clone, Debug, ValueEnum)]
pub enum HelpTopic {
    /// The `.ws` YAML schema and configuration reference.
    Config,
    /// The `ws clip` credential-mapping schema, file locations, and configuration reference.
    Clip,
}

/// Arguments for workspace creation or reconnection.
#[derive(Debug, Args)]
pub struct UpArgs {
    /// Name of the tmux session to create or connect to.
    #[arg(value_name = "SESSION_NAME", value_parser = validate_session_name)]
    pub session_name: String,
}

/// Arguments for operations that target one tmux session.
#[derive(Debug, Args)]
pub struct SessionArgs {
    /// Name of the existing tmux session.
    #[arg(value_name = "SESSION_NAME", value_parser = validate_session_name)]
    pub session_name: String,
}

/// Arguments for terminating a workspace session.
#[derive(Debug, Args)]
pub struct DownArgs {
    /// Name of the tmux session; omitted only for confirmed current-session operation inside tmux.
    #[arg(value_name = "SESSION_NAME", value_parser = validate_session_name)]
    pub session_name: Option<String>,

    /// Explicitly confirm the destructive operation without an interactive prompt.
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for `ws clip NAMESPACE ITEM`.
#[derive(Debug, Args)]
pub struct ClipArgs {
    /// Logical credential namespace.
    pub namespace: String,
    /// Logical credential item.
    pub item: String,
}

fn validate_session_name(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("session name must not be empty".to_owned());
    }

    if value.chars().any(char::is_control) {
        return Err("session name must not contain control characters".to_owned());
    }

    // tmux reserves `:` and `.` to separate a session from a window or pane index in a target
    // specifier, so a session name containing either could be misinterpreted by tmux itself.
    if value.contains([':', '.']) {
        return Err("session name must not contain ':' or '.'".to_owned());
    }

    Ok(value.to_owned())
}
