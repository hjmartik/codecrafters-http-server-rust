#[derive(Clone, Copy, PartialEq)]
pub enum StatusCode {
    Ok = 200,
    NotFound = 404,
}

impl Into<u16> for StatusCode {
    fn into(self) -> u16 {
        self as u16
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
