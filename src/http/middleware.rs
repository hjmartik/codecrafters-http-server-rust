use crate::http::{request::Request, State};

use super::{header::Headers, response::Response, router::Handler, status::StatusCode};

pub fn content_length(handler: Handler) -> Handler {
    Box::new(move |request: Request, state: State| {
        let resp = handler(request, state);
        Box::pin(async move {
            let mut resp = resp.await;
            let len = match &resp.body {
                Some(body) => body.data.len(),
                None => 0,
            };
            if len > 0 {
                let mut headers = resp.headers.take().unwrap_or(Headers::new());
                headers.insert("Content-Length".to_string(), len.to_string());
                resp.headers = Some(headers);
            }
            resp
        })
    })
}

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

        let resp = handler(request, state.clone());
        if let Some(content_encoding) = content_encoding {
            return Box::pin(async move {
                let mut resp = resp.await;
                let body = match resp.body.take() {
                    Some(body) => body,
                    None => return resp,
                };
                let encoder = state.encoder(content_encoding.as_str()).unwrap();
                let encoded_body = match encoder(body) {
                    Ok(encoded_body) => encoded_body,
                    Err(_) => return Response::from_status(StatusCode::Internal),
                };
                resp.body = Some(encoded_body);

                let mut headers = resp.headers.take().unwrap_or(Headers::new());
                headers.insert("Content-Encoding".to_string(), content_encoding);
                resp.headers = Some(headers);
                resp
            });
        }
        resp
    })
}
