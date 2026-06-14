use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Command, ChildStdin, ChildStdout};

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn request(&self, message: &str) -> Result<String>;
}

// --- Stdio transport ---

pub struct StdioTransport {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl StdioTransport {
    pub async fn new(command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("failed to capture stdin"))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("failed to capture stdout"))?;

        Ok(Self {
            stdin: tokio::sync::Mutex::new(stdin),
            stdout: tokio::sync::Mutex::new(BufReader::new(stdout)),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn request(&self, message: &str) -> Result<String> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        drop(stdin);

        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();
        stdout.read_line(&mut line).await?;
        Ok(line.trim().to_string())
    }
}

// --- HTTP transport ---

pub struct HttpTransport {
    url: String,
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn request(&self, message: &str) -> Result<String> {
        let resp = self.client
            .post(&self.url)
            .header("content-type", "application/json")
            .body(message.to_string())
            .send()
            .await?;
        let text = resp.text().await?;
        Ok(text)
    }
}
