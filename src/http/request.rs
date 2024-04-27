use crate::http::header::Headers;
use crate::http::helpers::{self, CursorError};
use bytes::{Buf, BufMut, BytesMut};
use std::{
    io::Cursor,
    str::{self, Utf8Error},
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug)]
pub struct Request {
    pub path: String,
    pub headers: Headers,
}

impl Request {
    pub fn validate(cursor: &mut Cursor<&[u8]>) -> Result<(), RequestError> {
        // start line present
        helpers::get_until_crlf(cursor)?;
        // are there headers
        loop {
            let line = helpers::get_until_crlf(cursor)?;
            if line.is_empty() {
                break;
            }
        }
        Ok(())
    }

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Request, RequestError> {
        let request_line = str::from_utf8(helpers::get_until_crlf(cursor)?)?;
        let path = request_line
            .split(' ')
            .nth(1)
            .ok_or(RequestError::Invalid)?
            .to_owned();
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

        Ok(Request { path, headers })
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

    pub fn request_from_buffer(&mut self) -> Result<Request, RequestError> {
        let mut cursor = Cursor::new(&self.buf[..]);
        let _ = Request::validate(&mut cursor)?;
        cursor.set_position(0);
        let request = Request::parse(&mut cursor)?;
        self.buf.advance(cursor.position() as usize);
        Ok(request)
    }

    pub async fn read_request<R>(&mut self, reader: &mut R) -> Result<Request, RequestParserError>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            match self.request_from_buffer() {
                Ok(request) => {
                    println!("request: {:?}", request);
                    return Ok(request);
                }
                Err(RequestError::Incomplete) => {
                    if 0 == reader.read_buf(&mut self.buf).await? {
                        return Err(RequestParserError::Disconnect);
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
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
        let request = parser.request_from_buffer()?;
        assert_eq!(request.path, "/index.html");
        Ok(())
    }

    #[test]
    fn no_headers() -> Result<(), anyhow::Error> {
        let mut parser = RequestParser::new();
        parser.put(&b"GET /index.html HTTP/1.1\r\n\r\n"[..]);
        let request = parser.request_from_buffer()?;
        assert!(request.headers.is_empty());
        Ok(())
    }
}
