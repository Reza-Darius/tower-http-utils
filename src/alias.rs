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
///
/// This type is appropiate for the "leaf" service inside a tower stack
pub type Body = UnsyncBoxBody<Bytes, BoxError>;

/// A type erased [`tower::Service`], intended to be used as a function return type and argument
/// when used with a [hyper server](https://docs.rs/hyper/latest/hyper/server/index.html)
///
/// currently, this alias uses [`anyhow`] because that is the only way i could reliably circumvent
/// this [compiler bug](https://github.com/rust-lang/rust/issues/102211#event-24854862820)
/// ```
/// pub fn setup_service(config: &Config) -> HyperService {
///     ServiceBuilder::new()
///         .layer(TraceLayer::new_for_http())
///         .layer(TimeoutLayer::with_status_code(
///             StatusCode::REQUEST_TIMEOUT,
///             Duration::from_secs(20),
///         ))
///         .layer(CatchPanicLayer::custom(handle_panic))
///         .layer(RateLimitLayer::new(config.global.limit))
///         .layer(RequestBodyLimitLayer::new(4096))
///         .layer(NormalizePathLayer::trim_trailing_slash())
///         .layer(AddrServiceLayer::new(peers))
///         .service(hyper_client::UpstreamService::new())
///         .map_err(anyhow::Error::from_boxed)
///         .map_response(|resp: http::Response<_>| resp.map(|body| body.boxed_unsync()))
///         .boxed_clone()
/// }
/// ```
pub type HyperService = BoxCloneService<Request<Incoming>, Response<Body>, anyhow::Error>;
