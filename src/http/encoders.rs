use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::prelude::*;

use super::Body;

pub type EncoderFn = Box<dyn Fn(Body) -> Result<Body, anyhow::Error> + Send + Sync + 'static>;

pub fn gzip_encoder(body: Body) -> Result<Body, anyhow::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&body.data)?;
    encoder.flush()?;
    Ok(Body {
        data: encoder.finish()?,
    })
}
