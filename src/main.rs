#![doc = "Command-line tooling for configured tmux workspaces."]
#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod app;
mod cli;
mod config;
mod credentials;
mod error;
mod help;
mod lifecycle;
mod terminal;
mod tmux;

use clap::Parser;

use crate::app::{App, AppDependencies};
use crate::cli::Cli;
use crate::error::render_error;
use crate::terminal::launcher::UnavailableTerminalLauncher;
use crate::tmux::client::TmuxClient;

fn main() {
    let cli = Cli::parse();
    let app = App::new(AppDependencies {
        tmux: TmuxClient::default(),
        terminal_launcher: Box::new(UnavailableTerminalLauncher),
    });

    match app.execute(cli) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{}", render_error(&error));
            std::process::exit(error.exit_status().code());
        }
    }
}
