use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
};

use self::bytes::Framer;

pub mod bytes;

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

struct Connection {
    stream: BufWriter<TcpStream>,
    framer: Framer,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream: BufWriter::new(stream),
            framer: Framer::new(),
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        Ok(())
    }
}

async fn handle_client(stream: TcpStream) -> anyhow::Result<()> {
    let mut conn = Connection::new(stream);
    conn.write(b"HTTP/1.1 200 OK\r\n\r\n").await?;
    Ok(())
}
