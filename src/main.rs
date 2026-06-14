mod cli;
mod config;
mod agent;
mod llm;
mod tools;
mod session;
mod tui;
mod mcp;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("aibsd=info".parse()?))
        .init();

    let cli = cli::Cli::parse();

    let cfg = config::Config::load(&cli.config)?;
    tracing::info!("aibsd v{} starting", env!("CARGO_PKG_VERSION"));

    match cli.command {
        cli::Command::Tui => {
            let mcp_connections = build_mcp_connections(&cfg).await?;
            let mut terminal = tui::init_terminal()?;
            let mut app = tui::App::new(&cfg, mcp_connections);
            let res = app.run(&mut terminal);
            tui::restore_terminal()?;
            res?;
        }
        cli::Command::Run { prompt } => {
            let provider = llm::create_provider(&cfg.llm)?;
            let mut registry = tools::registry::builtin_tools();
            let mcp_connections = build_mcp_connections(&cfg).await?;
            for conn in &mcp_connections {
                for mt in &conn.tools {
                    registry.register(Box::new(mcp::McpToolAdapter::new(
                        std::sync::Arc::clone(&conn.client),
                        mt.clone(),
                    )));
                }
            }
            let mut agent = agent::Agent::new(provider, registry, &cfg);
            let response = agent.run(&prompt, None).await?;
            println!("{}", response);
        }
        cli::Command::Serve { .. } => {
            anyhow::bail!("serve mode not yet implemented");
        }
    }

    Ok(())
}

async fn build_mcp_connections(cfg: &config::Config) -> Result<Vec<tui::McpConnection>> {
    let mut connections = Vec::new();

    for server in &cfg.mcp.servers {
        let client: mcp::McpClient = match server.transport.as_str() {
            "stdio" => {
                let cmd = server.command.as_deref()
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{}': command required for stdio", server.name))?;
                let transport = mcp::StdioTransport::new(cmd, &server.args).await?;
                mcp::McpClient::new(Box::new(transport))
            }
            "http" => {
                let url = server.url.as_deref()
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{}': url required for http", server.name))?;
                let transport = mcp::HttpTransport::new(url);
                mcp::McpClient::new(Box::new(transport))
            }
            other => anyhow::bail!("MCP server '{}': unknown transport '{}'", server.name, other),
        };

        client.initialize().await?;
        let server_name = client.server_name().await;
        let tools = client.list_tools().await?;

        tracing::info!(
            "MCP connected: {} ({} tools)",
            server_name,
            tools.len()
        );

        connections.push(tui::McpConnection {
            client: std::sync::Arc::new(tokio::sync::Mutex::new(client)),
            tools,
        });
    }

    Ok(connections)
}
