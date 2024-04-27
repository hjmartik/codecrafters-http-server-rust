use std::io::Cursor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CursorError {
    #[error("incomplete data")]
    Incomplete,

    #[error("invalid data")]
    Invalid,
}

pub(crate) fn get_until_crlf<'a>(cursor: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], CursorError> {
    let start = cursor.position() as usize;
    let slice = cursor.get_ref();
    let end = slice.len();
    if end < 2 {
        return Err(CursorError::Incomplete);
    }

    for i in start..(end - 1) {
        if slice[i] == b'\r' && slice[i + 1] == b'\n' {
            cursor.set_position((i + 2) as u64);
            return Ok(&cursor.get_ref()[start..i]);
        }
    }

    Err(CursorError::Incomplete)
}
