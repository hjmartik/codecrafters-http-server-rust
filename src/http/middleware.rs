use crate::http::{request::Request, State};

use super::{header::Headers, router::Handler};

pub fn content_encoding(handler: Handler) -> Handler {
    Box::new(move |request: Request, state: State| { 
        let encoding = request
            .metadata
            .headers
            .get("Accept-Encoding")
            .map(str::to_string);
        let mut content_encoding = None;

        if let Some(encoding) = encoding {
            if state.supported_encoding(encoding.as_str()) {
                content_encoding = Some(encoding);
            }
        }

        let resp = handler(request, state);
        if let Some(content_encoding) = content_encoding {
            return Box::pin(async move {
                let mut resp = resp.await;
                let mut headers = resp.headers.take().unwrap_or(Headers::new());
                headers.insert("Content-Encoding".to_string(), content_encoding);
                resp.headers = Some(headers);
                resp
            });
        }
        resp
    })
}
