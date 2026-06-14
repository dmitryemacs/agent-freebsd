use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aibsd", version, about = "FreeBSD-first AI coding agent")]
pub struct Cli {
    #[arg(short, long, default_value = "~/.config/aibsd/config.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive TUI mode (default)
    Tui,
    /// Run a single prompt non-interactively
    Run {
        #[arg(required = true)]
        prompt: String,
    },
    /// Start HTTP server mode
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
    },
}
