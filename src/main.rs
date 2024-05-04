use anyhow::{anyhow, Context, Result};
use std::env;
use tokio::net::TcpListener;

use http_server_starter_rust::http;

const DEFAULT_PORT: u32 = 4221;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {}", addr))?;

    let mut args = env::args().skip(1);
    let dir_flag = match args.next() {
        Some(flag) if flag == "--directory" => true,
        Some(_) => {
            println!("--directory is the only valid flag");
            return Err(anyhow!("invalid input"));
        }
        None => false,
    };

    let mut directory = None;
    if dir_flag {
        if let Some(dir) = args.next() {
            directory = Some(dir);
        } else {
            println!("Usage: --directory <directory>");
            return Err(anyhow!("missing directory name"));
        }
    }

    http::run_server(listener, directory).await;
    Ok(())
}
