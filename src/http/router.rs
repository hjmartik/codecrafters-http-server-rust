use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::http::request::Request;
use crate::http::response::Response;
use crate::http::State;

use super::handlers;

pub type BoxResponseFuture = Pin<Box<dyn Future<Output = Response> + Send + 'static>>;
type Handler = Box<dyn Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static>;

type HandlerMap = HashMap<String, Handler>;

#[derive(Clone)]
pub struct Router(Arc<RouterInner>);

pub struct RouterInner {
    exact: HandlerMap,
    starts_with: Vec<(String, Handler)>,
}

impl RouterInner {
    pub fn exact_route<C>(mut self, mut path: &str, callback: C) -> Self
    where
        C: Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static,
    {
        if path != "/" {
            path = path.strip_suffix("/").unwrap_or(path);
        }
        self.exact.insert(path.to_string(), Box::new(callback));
        self
    }

    pub fn starts_with_route<C>(mut self, path: &str, callback: C) -> Self
    where
        C: Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static,
    {
        self.starts_with
            .push((path.to_string(), Box::new(callback)));
        self
    }

    pub fn build(self) -> Router {
        Router(Arc::new(self))
    }
}

impl Router {
    pub fn builder() -> RouterInner {
        RouterInner {
            exact: HashMap::new(),
            starts_with: Vec::new(),
        }
    }

    pub async fn handle(&self, request: Request, state: State) -> Response {
        let path = &request.path;
        let key = match path.as_str() {
            "/" => "/",
            _ => path.strip_suffix("/").unwrap_or(path),
        };

        if let Some(handler) = self.0.exact.get(key) {
            return handler(request, state).await;
        }

        for (prefix, handler) in &self.0.starts_with {
            if path.starts_with(prefix) {
                return handler(request, state).await;
            }
        }

        handlers::not_found_handler(request, state).await
    }
}
