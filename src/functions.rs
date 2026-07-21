//! helper functions
use std::any::Any;

use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::{BodyExt, Empty, Full};
use tower::BoxError;

use crate::alias::{Body, SvcBoxFut};

/// helper function to safely clone a service, see comment
///
/// Services are permitted to panic if call is invoked without obtaining [`Poll::Ready(Ok(())`)] from poll_ready.
/// You should therefore be careful when cloning services for example to move them into boxed futures.
/// Even though the original service is ready, the clone might not be.
pub fn svc_clone<S: Clone + Sized>(inner: &mut S) -> S {
    let clone = inner.clone();
    // take the service that was ready
    std::mem::replace(inner, clone)
}

/// construct an off-hand, type erased error for a service that returns [`SvcBoxFut`]
///
/// this is useful for early returns
///```
/// impl<B> Service<Request<B>> for ConnectionService<B>
/// where
///     B: hyper::body::Body + Send + 'static + Unpin,
///     B::Data: Send,
///     B::Error: Into<BoxError>,
/// {
///     type Response = Response<Body>;
///     type Error = BoxError;
///     type Future = SvcBoxFut<Self::Response, Self::Error>;
/// 
///     fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
///         Poll::Ready(Ok(()))
///     }
/// 
///     fn call(&mut self, req: Request<B>) -> Self::Future {
///         let Some(peer) = req.extensions().get::<Peer>().cloned() else {
///             return boxfut_err("no Peer info found on request");
///         };
///
///         //..
///
///         svc.call(req)
///     }
/// }
///```
pub fn boxfut_err<R>(e: impl std::fmt::Display) -> SvcBoxFut<R, BoxError> {
    let err: BoxError = e.to_string().into();
    Box::pin(async { Err(err) })
}

/// construct an [`http::Response`] for a service that returns [`SvcBoxFut`]
pub fn boxfut_res<E>(status: StatusCode) -> SvcBoxFut<Response<Body>, E> {
    let resp = response(status);
    Box::pin(async { Ok(resp) })
}

/// handler for using with [tower_http's panic handler](https://docs.rs/tower-http/latest/tower_http/catch_panic/index.html) panic handler middleware
pub fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic message".to_string()
    };
    tracing::error!(details, "request caused a panic");

    response(StatusCode::INTERNAL_SERVER_ERROR)
}

/// helper function to build a response with a specific body
pub fn response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(empty())
        .expect("the values are hard coded")
}


/// consturcts an empty [`Body`] 
pub fn empty() -> Body {
    Empty::<Bytes>::new()
        // .map_err(|never| match never {})
        .map_err(Into::into)
        .boxed_unsync()
}

/// consturcts a [`Body`] with data
pub fn full(chunk: impl Into<Bytes>) -> Body {
    Full::new(chunk.into()).map_err(Into::into).boxed_unsync()
}
