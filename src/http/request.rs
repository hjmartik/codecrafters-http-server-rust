use bytes::{Buf, BufMut, BytesMut};
use std::{
    io::Cursor,
    str::{self, Utf8Error},
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Request {
    pub path: String,
}

impl Request {
    pub fn validate(cursor: &mut Cursor<&[u8]>) -> Result<(), RequestError> {
        get_until_crlf(cursor)?;
        Ok(())
    }

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Request, RequestError> {
        let request_line = str::from_utf8(get_until_crlf(cursor)?)?;
        let path = request_line
            .split(' ')
            .nth(1)
            .ok_or(RequestError::Invalid)?
            .to_owned();
        Ok(Request { path })
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
                Ok(request) => return Ok(request),
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

fn get_until_crlf<'a>(cursor: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], RequestError> {
    let start = cursor.position() as usize;
    let slice = cursor.get_ref();
    let end = slice.len();
    if end == 0 {
        return Err(RequestError::Incomplete);
    }

    for i in start..(end - 1) {
        if slice[i] == b'\r' && slice[i + 1] == b'\n' {
            cursor.set_position((i + 2) as u64);
            return Ok(&cursor.get_ref()[start..i]);
        }
    }

    Err(RequestError::Incomplete)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn request_path() -> Result<(), anyhow::Error> {
        let mut parser = RequestParser::new();
        parser.put(
            &b"GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1"[..],
        );
        let request = parser.request_from_buffer()?;
        assert_eq!(request.path, "/index.html");
        Ok(())
    }
}
