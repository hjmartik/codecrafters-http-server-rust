use tokio::io::AsyncRead;

use crate::http::header::Headers;
use crate::http::status::StatusCode;
use crate::http::Body;

pub struct Response {
    pub status: StatusCode,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
}

impl Response {
    pub fn from_reader<R>(status: StatusCode, headers: Headers, reader: R) -> Self
    where
        R: AsyncRead + Send + Sync + 'static,
    {
        Self {
            status,
            headers: Some(headers),
            body: Some(Body::Reader(Box::new(reader))),
        }
    }

    pub fn from_data(status: StatusCode, headers: Headers, data: Vec<u8>) -> Self {
        Self {
            status,
            headers: Some(headers),
            body: Some(Body::Data(data)),
        }
    }

    pub fn from_status(status: StatusCode) -> Self {
        let mut headers = Headers::new();
        let status_text = status.to_string();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        headers.insert_header_line(format!("Content-Length: {}", status_text.len()));
        let body = status_text.into_bytes();
        Self {
            status,
            headers: Some(headers),
            body: Some(Body::Data(body)),
        }
    }
}
