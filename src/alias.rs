//! type aliases for tower services and hyper
use std::pin::Pin;

use bytes::Bytes;
use http::Response;
use tower::BoxError;

use http::Request;
use http_body_util::combinators::UnsyncBoxBody;
use hyper::body::Incoming;
use tower::util::BoxCloneService;


/// Convenience type suitable for Futures returned by [`tower::Service`] implementations
pub type SvcBoxFut<R, E> =
    Pin<Box<dyn Future<Output = std::result::Result<R, E>> + Send + 'static>>;

/// An [`hyper::body::Body`] type thats suitable as a "catch-all" body type inside an application
pub type Body = UnsyncBoxBody<Bytes, BoxError>;

/// A type erased [`tower::Service`], intended to be used as a function return type and argument
/// when used with a [hyper server](https://docs.rs/hyper/latest/hyper/server/index.html)
///
/// currently, this alias uses [`anyhow`] because that is the only way i could reliably circumvent
/// this [compiler bug](https://github.com/rust-lang/rust/issues/102211#event-24854862820)
pub type HyperService = BoxCloneService<Request<Incoming>, Response<Body>, anyhow::Error>;
