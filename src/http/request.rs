use crate::http::header::Headers;
use crate::http::helpers::{self, CursorError};
use crate::http::Body;
use bytes::{Buf, BufMut, BytesMut};
use std::{
    io::Cursor,
    str::{self, Utf8Error},
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::Method;

pub struct Request {
    pub metadata: Metadata,
    pub body: Option<Body>,
}

pub struct Metadata {
    pub method: Method,
    pub path: String,
    pub headers: Headers,
}

impl Metadata {
    pub fn new(method: Method, path: String, headers: Headers) -> Self {
        Metadata {
            method,
            path,
            headers,
        }
    }
}

impl Metadata {
    pub fn validate(cursor: &mut Cursor<&[u8]>) -> Result<(), RequestError> {
        helpers::get_until_crlf(cursor)?;
        while !helpers::get_until_crlf(cursor)?.is_empty() {}
        Ok(())
    }

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Metadata, RequestError> {
        let request_line = str::from_utf8(helpers::get_until_crlf(cursor)?)?;

        let mut splitted = request_line.split(' ');
        let method = match splitted.next().ok_or(RequestError::Invalid)? {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => return Err(RequestError::Invalid),
        };

        let path = splitted.next().ok_or(RequestError::Invalid)?.to_owned();

        let mut headers = Headers::new();
        loop {
            let header_line = str::from_utf8(helpers::get_until_crlf(cursor)?)?;
            if header_line.is_empty() {
                break;
            }
            let (key, value) = header_line.split_once(':').ok_or(RequestError::Invalid)?;
            let value = value.trim();
            headers.insert(key.to_owned(), value.to_owned());
        }

        Ok(Metadata::new(method, path, headers))
    }
}

pub struct RequestParser {
    buf: BytesMut,
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("incomplete request")]
    Incomplete,

    #[error("invalid request")]
    Invalid,
}

impl From<CursorError> for RequestError {
    fn from(err: CursorError) -> Self {
        match err {
            CursorError::Invalid => RequestError::Invalid,
            CursorError::Incomplete => RequestError::Incomplete,
        }
    }
}

impl From<Utf8Error> for RequestError {
    fn from(_: Utf8Error) -> Self {
        RequestError::Invalid
    }
}

#[derive(Error, Debug)]
pub enum RequestParserError {
    #[error("reader did not produce a complete request")]
    Disconnect,

    #[error("io error from reader")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    RequestError(#[from] RequestError),
}

impl RequestParser {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(2 * 1024),
        }
    }

    fn put<B: Buf>(&mut self, src: B) {
        self.buf.put(src);
    }

    pub fn metadata_from_buffer(&mut self) -> Result<Metadata, RequestError> {
        let mut cursor = Cursor::new(&self.buf[..]);
        Metadata::validate(&mut cursor)?;
        cursor.set_position(0);
        let metadata = Metadata::parse(&mut cursor)?;
        self.buf.advance(cursor.position() as usize);
        Ok(metadata)
    }

    pub async fn read_request<R>(&mut self, reader: &mut R) -> Result<Request, RequestParserError>
    where
        R: AsyncRead + Unpin,
    {
        let metadata;
        loop {
            match self.metadata_from_buffer() {
                Ok(md) => {
                    metadata = md;
                    break;
                }
                Err(RequestError::Incomplete) => {
                    if 0 == reader.read_buf(&mut self.buf).await? {
                        return Err(RequestParserError::Disconnect);
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        if metadata.method == Method::GET {
            return Ok(Request {
                metadata,
                body: None,
            });
        }

        let content_length: usize = metadata
            .headers
            .get("Content-Length")
            .ok_or(RequestError::Invalid)?
            .parse()
            .map_err(|_| RequestError::Invalid)?;

        let mut remaining = self.buf.remaining();
        // timeout needed...
        while remaining < content_length {
            remaining += reader.read_buf(&mut self.buf).await?;
        }
        let data = self.buf.copy_to_bytes(content_length).to_vec();

        return Ok(Request {
            metadata,
            body: Some(Body::Data(data)),
        });
    }

    pub fn buffer_is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn request_path() -> Result<(), anyhow::Error> {
        let mut parser = RequestParser::new();
        parser.put(
            &b"GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\n\r\n"
                [..],
        );
        let metadata = parser.metadata_from_buffer()?;
        assert_eq!(metadata.path, "/index.html");
        Ok(())
    }

    #[test]
    fn no_headers() -> Result<(), anyhow::Error> {
        let mut parser = RequestParser::new();
        parser.put(&b"GET /index.html HTTP/1.1\r\n\r\n"[..]);
        let metadata = parser.metadata_from_buffer()?;
        assert!(metadata.headers.is_empty());
        Ok(())
    }
}
