use crate::http::{request::Request, State};

use super::{header::Headers, router::Handler};

pub fn content_encoding(handler: Handler) -> Handler {
    Box::new(move |request: Request, state: State| {
        let content_encoding = match request.metadata.headers.get("Accept-Encoding") {
            Some(encodings) => encodings
                .split(",")
                .map(|s| s.trim())
                .find(|encoding| state.supported_encoding(encoding))
                .map(str::to_string),
            None => None,
        };

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
