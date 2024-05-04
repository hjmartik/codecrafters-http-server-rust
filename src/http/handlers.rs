use std::path::Path;

use crate::http::{header::Headers, status::StatusCode};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use super::router::BoxResponseFuture;
use super::{request::Request, response::Response};
use super::State;

pub fn echo_handler(request: Request, _state: State) -> BoxResponseFuture {
    // error handling...
    Box::pin(async move {
        let echo = request.path.strip_prefix("/echo/").unwrap();
        let mut headers = Headers::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        headers.insert_header_line(format!("Content-Length: {}", echo.len()));
        Response::from_data(StatusCode::Ok, headers, echo.as_bytes().to_vec())
    })
}

pub fn ok_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::Ok) })
}

pub fn not_found_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::NotFound) })
}

pub fn internal_erro_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::Internal) })
}

pub fn user_agent_handler(request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async move {
        let user_agent = request.headers.get("User-Agent").unwrap_or("");
        let mut headers = Headers::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        headers.insert_header_line(format!("Content-Length: {}", user_agent.len()));
        Response::from_data(StatusCode::Ok, headers, user_agent.as_bytes().to_vec())
    })
}

pub fn file_handler(request: Request, state: State) -> BoxResponseFuture {
    Box::pin(async move {
        let file_path = match request.path.strip_prefix("/files/") {
            Some(path) => path,
            None => return internal_erro_handler(request, state).await,
        };

        let dir = match state.file_dir() {
            Some(dir) => dir,
            None => return internal_erro_handler(request, state).await,
        };

        println!("directory: {}", dir);
        println!("file path: {}", file_path);
        let path = Path::new(dir).join(file_path);
        println!("path: {:?}", path);

        let mut file = match File::open(path).await {
            Ok(file) => file,
            Err(_) => return not_found_handler(request, state).await,
        };
        let mut buf = Vec::new();
        let length = match file.read_to_end(&mut buf).await {
            Ok(l) => l,
            Err(_) => return internal_erro_handler(request, state).await,
        };

        let mut headers = Headers::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert_header_line(format!("Content-Length: {}", length));

        Response::from_data(StatusCode::Ok, headers, buf)
    })
}
