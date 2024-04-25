use bytes::BytesMut;

pub struct Framer {
    buf: BytesMut,
}

impl Framer {
    pub fn new() -> Self {
        Framer {
            buf: BytesMut::with_capacity(2 * 1024),
        }
    }
}
