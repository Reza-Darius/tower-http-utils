//! [`http::Response`] extension utilities
//!
//! An example of a middleware service where we either want to "pass through" the response unaltered
//! or build a custom response in case of an error
//!
//! in order to stay generic over the underlying service, we need a wrapper like [`WrapBody`]
//! ```
//!impl<F, E, ResB> Future for RateLimitFuture<F>
//! where
//!     F: Future<Output = Result<Response<ResB>, E>>,
//!     E: Into<BoxError>,
//! {
//!     type Output = Result<Response<WrapBody<ResB>>, BoxError>;
//! 
//!     fn poll(
//!         self: std::pin::Pin<&mut Self>,
//!         cx: &mut std::task::Context<'_>,
//!     ) -> std::task::Poll<Self::Output> {
//!         let this = self.project();
//!         match this {
//!             EnumProj::Ok { fut } => fut
//!                 .poll(cx)
//!                 .map_err(Into::into)
//!                 // we wrap the underlying body using the extension trait
//!                 .map(|res| res.map(|resp| resp.map_body())),
//!             EnumProj::RateLimited => Poll::Ready(Ok(Response::build(StatusCode::TOO_MANY_REQUESTS, "too any requests"))),
//!             EnumProj::NoAddrFoun => Poll::Ready(Err("no addr found on request".into())),
//!         }
//!     }
//! }
//! ```
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::Full;
use hyper::body::{Frame, SizeHint};
use pin_project_lite::pin_project;

/// Extension trait to provide some useful utilities for working with tower http services that
/// often return multiple body types
///
/// extends [`http::Response`] with [`WrapBody`]
pub trait ResponseBodyExt<B> {
    /// maps a response's body to the ResponseBody wrapper type
    ///
    /// intended for the happy path
    fn map_body(self) -> Response<WrapBody<B>>;

    /// builds a response with a response body
    fn build(status: StatusCode, body: impl Into<Bytes>) -> Response<WrapBody<B>>;

    /// builds a response with an empty response body
    ///
    /// useful for creating ad-hoc error responses
    fn empty(status: StatusCode) -> Response<WrapBody<B>>;

    /// builds a response with an empty response body, and 200 status code
    fn ok() -> Response<WrapBody<B>>;
}

impl<B> ResponseBodyExt<B> for Response<B> {
    #[inline]
    fn map_body(self) -> Response<WrapBody<B>> {
        self.map(WrapBody::wrap)
    }

    #[inline]
    fn build(status: StatusCode, body: impl Into<Bytes>) -> Response<WrapBody<B>> {
        Response::builder()
            .status(status)
            .body(WrapBody::new(body))
            .unwrap()
    }

    #[inline]
    fn empty(status: StatusCode) -> Response<WrapBody<B>> {
        Response::builder()
            .status(status)
            .body(WrapBody::empty())
            .unwrap()
    }

    #[inline]
    fn ok() -> Response<WrapBody<B>> {
        Response::empty(StatusCode::OK)
    }
}

pin_project! {
    /// a body which is generic over a wrapped body while being able to hold a body of a different
    /// type, this is similar to [`http_body_util::Either`].
    ///
    /// This type is most useful for middleware, for constructing concrete "leaf" services consider
    /// [`Body`](super::alias::Body)
    #[project = BodyProj]
    pub enum WrapBody<B> {
        #[doc(hidden)]
        Full {
            #[pin]
            body: Full<Bytes>,
        },
        #[doc(hidden)]
        Empty,
        #[doc(hidden)]
        Wrapped {
            #[pin]
            body: B
        }
    }
}

impl<B> WrapBody<B> {
    /// create a new body data
    pub fn new(data: impl Into<Bytes>) -> Self {
        WrapBody::Full {
            body: Full::new(data.into()),
        }
    }

    /// create a empty body
    pub fn empty() -> Self {
        WrapBody::Empty
    }

    /// wraps another body, use this if you want to pass a generic body unaltered
    pub fn wrap(body: B) -> Self {
        WrapBody::Wrapped { body }
    }
}

impl<B> hyper::body::Body for WrapBody<B>
where
    B: hyper::body::Body<Data = Bytes>,
{
    type Data = Bytes;
    type Error = B::Error;

    #[inline]
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            BodyProj::Full { body } => body.poll_frame(cx).map_err(|e| match e {}),
            BodyProj::Wrapped { body } => body.poll_frame(cx),
            BodyProj::Empty => Poll::Ready(None),
        }
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        match &self {
            WrapBody::Full { body } => body.is_end_stream(),
            WrapBody::Wrapped { body } => body.is_end_stream(),
            WrapBody::Empty => true,
        }
    }

    #[inline]
    fn size_hint(&self) -> SizeHint {
        match &self {
            WrapBody::Full { body } => body.size_hint(),
            WrapBody::Wrapped { body } => body.size_hint(),
            WrapBody::Empty => SizeHint::with_exact(0),
        }
    }
}
