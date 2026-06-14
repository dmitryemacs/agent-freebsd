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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_tui() {
        let cli = Cli::parse_from(["aibsd", "tui"]);
        assert!(matches!(cli.command, Command::Tui));
        assert_eq!(cli.config, "~/.config/aibsd/config.toml");
    }

    #[test]
    fn test_cli_run() {
        let cli = Cli::parse_from(["aibsd", "run", "hello"]);
        match &cli.command {
            Command::Run { prompt } => assert_eq!(prompt, "hello"),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn test_cli_run_with_config() {
        let cli = Cli::parse_from(["aibsd", "--config", "/tmp/test.toml", "run", "test"]);
        assert_eq!(cli.config, "/tmp/test.toml");
        match &cli.command {
            Command::Run { prompt } => assert_eq!(prompt, "test"),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn test_cli_serve() {
        let cli = Cli::parse_from(["aibsd", "serve"]);
        match &cli.command {
            Command::Serve { bind } => assert_eq!(bind, "127.0.0.1:8080"),
            _ => panic!("expected Serve"),
        }
    }

    #[test]
    fn test_cli_serve_custom_bind() {
        let cli = Cli::parse_from(["aibsd", "serve", "-b", "0.0.0.0:9090"]);
        match &cli.command {
            Command::Serve { bind } => assert_eq!(bind, "0.0.0.0:9090"),
            _ => panic!("expected Serve"),
        }
    }
}
