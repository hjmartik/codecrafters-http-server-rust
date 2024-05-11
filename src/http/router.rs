use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::http::request::Request;
use crate::http::response::Response;
use crate::http::State;

use super::{handlers, Method};

pub type BoxResponseFuture = Pin<Box<dyn Future<Output = Response> + Send + 'static>>;
pub type Handler = Box<dyn Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static>;
pub type Middleware = Box<dyn Fn(Handler) -> Handler + Send + Sync + 'static>;

struct HandlerEntry {
    handler: Handler,
    method: Method,
}

type HandlerMap = HashMap<String, HandlerEntry>;

#[derive(Clone)]
pub struct Router(Arc<RouterInner>);

pub struct RouterInner {
    exact: HandlerMap,
    starts_with: Vec<(String, HandlerEntry)>,
    pre_middleware: Vec<Middleware>,
}

impl RouterInner {
    pub fn exact_route<H>(mut self, mut path: &str, method: Method, handler: H) -> Self
    where
        H: Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static,
    {
        if path != "/" {
            path = path.strip_suffix("/").unwrap_or(path);
        }

        let mut handler: Handler = Box::new(handler);
        for middleware in &self.pre_middleware {
            handler = middleware(handler);
        }

        self.exact
            .insert(path.to_string(), HandlerEntry { handler, method });
        self
    }

    pub fn starts_with_route<H>(mut self, path: &str, method: Method, handler: H) -> Self
    where
        H: Fn(Request, State) -> BoxResponseFuture + Send + Sync + 'static,
    {
        let mut handler: Handler = Box::new(handler);
        for middleware in &self.pre_middleware {
            handler = middleware(handler);
        }
        self.starts_with.push((
            path.to_string(),
            HandlerEntry {
                handler,
                method,
            },
        ));
        self
    }

    pub fn add_pre_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Fn(Handler) -> Handler + Send + Sync + 'static,
    {
        self.pre_middleware.push(Box::new(middleware));
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
            pre_middleware: Vec::new(),
        }
    }

    pub async fn handle(&self, request: Request, state: State) -> Response {
        let mut route_seen_flag = false;
        let path = &request.metadata.path;
        let key = match path.as_str() {
            "/" => "/",
            _ => path.strip_suffix("/").unwrap_or(path),
        };

        if let Some(handler_entry) = self.0.exact.get(key) {
            route_seen_flag = true;
            if handler_entry.method == request.metadata.method {
                return (handler_entry.handler)(request, state).await;
            }
        }

        for (prefix, handler_entry) in &self.0.starts_with {
            if path.starts_with(prefix) {
                route_seen_flag = true;
                if handler_entry.method == request.metadata.method {
                    return (handler_entry.handler)(request, state).await;
                }
            }
        }

        if route_seen_flag {
            return handlers::method_not_allowed_handler(request, state).await;
        }
        handlers::not_found_handler(request, state).await
    }
}
