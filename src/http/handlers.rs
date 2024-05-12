use std::path::Path;

use crate::http::Body;
use crate::http::{header::Headers, status::StatusCode};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::router::BoxResponseFuture;
use super::State;
use super::{request::Request, response::Response};

pub fn echo_handler(request: Request, _state: State) -> BoxResponseFuture {
    // error handling...
    Box::pin(async move {
        let echo = request.metadata.path.strip_prefix("/echo/").unwrap();
        let mut headers = Headers::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        Response::from_data(StatusCode::Ok, headers, echo.as_bytes().to_vec())
    })
}

pub fn ok_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::Ok) })
}

pub fn not_found_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::NotFound) })
}

pub fn internal_error_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::Internal) })
}

pub fn method_not_allowed_handler(_request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async { Response::from_status(StatusCode::MethodNotAllowed) })
}

pub fn user_agent_handler(request: Request, _state: State) -> BoxResponseFuture {
    Box::pin(async move {
        let user_agent = request.metadata.headers.get("User-Agent").unwrap_or("");
        let mut headers = Headers::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        Response::from_data(StatusCode::Ok, headers, user_agent.as_bytes().to_vec())
    })
}

pub fn file_get_handler(request: Request, state: State) -> BoxResponseFuture {
    Box::pin(async move {
        let file_path = match request.metadata.path.strip_prefix("/files/") {
            Some(path) => path,
            None => return internal_error_handler(request, state).await,
        };

        let dir = match state.file_dir() {
            Some(dir) => dir,
            None => return internal_error_handler(request, state).await,
        };

        let path = Path::new(dir).join(file_path);

        let mut file = match File::open(path).await {
            Ok(file) => file,
            Err(_) => return not_found_handler(request, state).await,
        };
        let mut buf = Vec::new();
        match file.read_to_end(&mut buf).await {
            Ok(_) => {},
            Err(_) => return internal_error_handler(request, state).await,
        };

        let mut headers = Headers::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        );

        Response::from_data(StatusCode::Ok, headers, buf)
    })
}

pub fn file_post_handler(mut request: Request, state: State) -> BoxResponseFuture {
    Box::pin(async move {
        let file_path = match request.metadata.path.strip_prefix("/files/") {
            Some(path) => path,
            None => return internal_error_handler(request, state).await,
        };

        let dir = match state.file_dir() {
            Some(dir) => dir,
            None => return internal_error_handler(request, state).await,
        };

        let path = Path::new(dir).join(file_path);

        let mut file = match File::create(path).await {
            Ok(file) => file,
            Err(_) => return internal_error_handler(request, state).await,
        };

        let data = match request.body.take() {
            Some(Body { data }) => data,
            _ => return internal_error_handler(request, state).await,
        };

        if let Err(_) = file.write_all(&data).await {
            return internal_error_handler(request, state).await;
        }

        if let Err(_) = file.flush().await {
            return internal_error_handler(request, state).await;
        }

        Response::from_status(StatusCode::Created)
    })
}
