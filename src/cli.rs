//! Command-line interface.

use clap::builder::styling;
use clap_verbosity_flag::{InfoLevel, Verbosity};

use crate::exe;

/// Serve a media collection over HTTP.
#[derive(Debug)]
#[derive(clap::Parser)]
#[command(author, version, about, long_about)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Execution mode.
    #[command(subcommand)]
    pub cmd: Command,

    /// Logging verbosity.
    #[command(flatten)]
    #[command(next_display_order = 100)]
    pub log: Verbosity<InfoLevel>,
}

/// Execution mode.
#[derive(Debug)]
#[derive(clap::Subcommand)]
#[command(
    styles = styling::Styles::styled()
        .header(styling::AnsiColor::BrightGreen.on_default().bold())
        .usage(styling::AnsiColor::BrightGreen.on_default().bold())
        .literal(styling::AnsiColor::BrightCyan.on_default().bold())
        .placeholder(styling::AnsiColor::BrightCyan.on_default())
)]
#[non_exhaustive]
pub enum Command {
    /// Start the HTTP server.
    #[command(visible_alias = "s")]
    Serve(Box<exe::serve::Cli>),
    /// Export the media collection.
    #[command(visible_alias = "export")]
    Dump(Box<exe::dump::Cli>),
    /// Import into the media collection.
    #[command(visible_alias = "import")]
    Load(Box<exe::load::Cli>),
}
