use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
};

use self::request::{Request, RequestParser, RequestParserError};

pub mod request;

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
    parser: RequestParser,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream: BufWriter::new(stream),
            parser: RequestParser::new(),
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn read_request(&mut self) -> Result<Option<Request>, RequestParserError> {
        match self.parser.read_request(&mut self.stream).await {
            Ok(request) => Ok(Some(request)),
            Err(e @ RequestParserError::Disconnect) => {
                if self.parser.buffer_is_empty() {
                    return Ok(None);
                }
                Err(e)
            }
            Err(e) => Err(e),
        }
    }
}

async fn handle_client(stream: TcpStream) -> anyhow::Result<()> {
    let mut conn = Connection::new(stream);
    let request = conn.read_request().await?;
    if let Some(request) = request {
        eprintln!("{}", request.path);
        match request.path.as_str() {
            "/" => conn.write(b"HTTP/1.1 200 OK\r\n\r\n").await?,
            _ => conn.write(b"HTTP/1.1 404 Not Found\r\n\r\n").await?,
        }
    }
    Ok(())
}
