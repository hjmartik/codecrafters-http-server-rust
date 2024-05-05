use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
};

use self::{
    request::{Request, RequestParser, RequestParserError},
    response::Response,
    router::Router,
};

pub mod handlers;
pub mod header;
pub mod helpers;
pub mod request;
pub mod response;
pub mod router;
pub mod status;

pub enum Body {
    Reader(Box<dyn AsyncRead + Send + Sync>),
    Data(Vec<u8>),
}

#[derive(PartialEq)]
pub enum Method {
    GET,
    POST,
}

#[derive(Clone)]
pub struct State(Arc<StateInner>);

struct StateInner {
    pub file_dir: Option<String>,
}

impl StateInner {
    fn file_dir(mut self, dir: String) -> Self {
        self.file_dir = Some(dir);
        self
    }

    fn build(self) -> State {
        State(Arc::new(self))
    }
}

impl State {
    fn builder() -> StateInner {
        StateInner { file_dir: None }
    }

    fn file_dir(&self) -> Option<&str> {
        self.0.file_dir.as_ref().map(|s| s.as_str())
    }
}

pub async fn run_server(listener: TcpListener, file_directory: Option<String>) {
    let mut state_builder = State::builder();

    let mut router_builder = Router::builder()
        .exact_route("/", Method::GET, handlers::ok_handler)
        .exact_route("/user-agent", Method::GET, handlers::user_agent_handler)
        .starts_with_route("/echo/", Method::GET, handlers::echo_handler);
    if let Some(dir) = file_directory {
        router_builder = router_builder
            .starts_with_route("/files/", Method::GET, handlers::file_get_handler)
            .starts_with_route("/files/", Method::POST, handlers::file_post_handler);
        state_builder = state_builder.file_dir(dir);
    }
    let router = router_builder.build();
    let state = state_builder.build();

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("Accepted connection from address: {}", addr);
                let state = state.clone();
                let router = router.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, router, state).await {
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
        Ok(())
    }

    pub async fn write_response(&mut self, response: Response) -> Result<(), std::io::Error> {
        let start_line = format!("HTTP/1.1 {}\r\n", response.status);
        self.write(start_line.as_bytes()).await?;
        if let Some(headers) = response.headers {
            for (header, value) in &headers {
                self.write(format!("{header}: {value}\r\n").as_bytes())
                    .await?;
            }
        }
        self.write("\r\n".as_bytes()).await?;
        match response.body {
            Some(Body::Data(data)) => {
                self.write(&data).await?;
            }
            Some(_) | None => {}
        }
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

async fn handle_client(stream: TcpStream, router: Router, state: State) -> anyhow::Result<()> {
    let mut conn = Connection::new(stream);
    while let Some(request) = conn.read_request().await? {
        let state = state.clone();
        let response = router.handle(request, state).await;
        conn.write_response(response).await?;
    }

    Ok(())
}
