use tokio::net::{TcpListener, TcpStream};

pub async fn run_server(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("Accepted connection from address: {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream).await {
                        eprintln!("Error with handling client: {:?}", e);
                    };
                });
            }
            Err(e) => eprintln!("Failed to accept connection {:?}", e),
        }
    }
}

async fn handle_client(_stream: TcpStream) -> anyhow::Result<()> {
    Ok(())
}
