use std::{collections::HashMap, sync::Arc};

use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
};

use self::{
    encoders::EncoderFn, request::{Request, RequestParser, RequestParserError}, response::Response, router::Router
};

pub mod handlers;
pub mod header;
pub mod helpers;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod status;
pub mod encoders;

pub struct Body {
    pub data: Vec<u8>,
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
    pub supported_encodings: HashMap<String, EncoderFn>,
}

impl StateInner {
    fn file_dir(mut self, dir: String) -> Self {
        self.file_dir = Some(dir);
        self
    }

    fn encoding<E>(mut self, encoding: String, encoder: E) -> Self
    where E: Fn(Body) -> Result<Body, anyhow::Error> + Send + Sync + 'static {
        self.supported_encodings.insert(encoding, Box::new(encoder));
        self
    }

    fn build(self) -> State {
        State(Arc::new(self))
    }
}

impl State {
    fn builder() -> StateInner {
        StateInner {
            file_dir: None,
            supported_encodings: HashMap::new(),
        }
    }

    fn file_dir(&self) -> Option<&str> {
        self.0.file_dir.as_ref().map(|s| s.as_str())
    }

    fn supported_encoding(&self, encoding: &str) -> bool {
        self.0.supported_encodings.contains_key(encoding)
    }

    fn encoder(&self, encoding: &str) -> Option<&EncoderFn> {
        self.0.supported_encodings.get(encoding)
    }
}

pub async fn run_server(listener: TcpListener, file_directory: Option<String>) {
    let mut state_builder = State::builder().encoding("gzip".to_string(), encoders::gzip_encoder);

    let mut router_builder = Router::builder()
        .add_middleware(middleware::content_encoding)
        .add_middleware(middleware::content_length)
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
            Some(Body { data }) => {
                self.write(&data).await?;
            }
            None => {}
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
