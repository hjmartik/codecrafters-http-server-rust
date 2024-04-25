use anyhow::{Context, Result};
use tokio::net::TcpListener;
use http_server_starter_rust::http;

const DEFAULT_PORT: u32 = 4221;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {}", addr))?;

    http::run_server(listener).await;
    Ok(())
}
