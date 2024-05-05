use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    NotFound = 404,
    MethodNotAllowed = 405,
    Internal = 500,
}

impl Into<u16> for StatusCode {
    fn into(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = match self {
            Self::Ok => "200 OK",
            Self::Created => "201 Created",
            Self::NotFound => "404 Not Found",
            Self::MethodNotAllowed => "405 Method Not Allowed",
            Self::Internal => "500 Internal Server Error",
        };
        write!(f, "{line}")
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn as_int() {
        assert_eq!(StatusCode::Ok as u16, 200);
    }
}
