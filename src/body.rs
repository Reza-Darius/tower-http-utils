//! [`http::Response`] extension utilities
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
/// extends [`http::Response`] with [`ResponseBody`]
pub trait ResponseBodyExt<B> {
    /// maps a response's body to the ResponseBody wrapper type
    fn map_body(self) -> Response<ResponseBody<B>>;

    /// builds a response with a response body
    fn build(status: StatusCode, body: impl Into<Bytes>) -> Response<ResponseBody<B>>;

    /// builds a response with an empty response body
    fn empty(status: StatusCode) -> Response<ResponseBody<B>>;

    /// builds a response with an empty response body, and 200 status code
    fn ok() -> Response<ResponseBody<B>>;
}

impl<B> ResponseBodyExt<B> for Response<B> {
    #[inline]
    fn map_body(self) -> Response<ResponseBody<B>> {
        self.map(ResponseBody::wrap)
    }

    #[inline]
    fn build(status: StatusCode, body: impl Into<Bytes>) -> Response<ResponseBody<B>> {
        Response::builder()
            .status(status)
            .body(ResponseBody::new(body))
            .unwrap()
    }

    #[inline]
    fn empty(status: StatusCode) -> Response<ResponseBody<B>> {
        Response::builder()
            .status(status)
            .body(ResponseBody::empty())
            .unwrap()
    }

    #[inline]
    fn ok() -> Response<ResponseBody<B>> {
        Response::empty(StatusCode::OK)
    }
}

pin_project! {
    /// a body which is generic over a wrapped body while being able to hold a body of a different
    /// type, this is similar to [`http_body_util::Either`]
    #[project = BodyProj]
    pub enum ResponseBody<B> {
        Full {
            #[pin]
            body: Full<Bytes>,
        },
        Empty,
        Wrapped {
            #[pin]
            body: B
        }
    }
}

impl<B> ResponseBody<B> {
    /// create a new body data
    pub fn new(data: impl Into<Bytes>) -> Self {
        ResponseBody::Full {
            body: Full::new(data.into()),
        }
    }

    /// create a empty body
    pub fn empty() -> Self {
        ResponseBody::Empty
    }

    /// wraps another body, use this if you want to pass a generic body unaltered
    pub fn wrap(body: B) -> Self {
        ResponseBody::Wrapped { body }
    }
}

impl<B> hyper::body::Body for ResponseBody<B>
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
            ResponseBody::Full { body } => body.is_end_stream(),
            ResponseBody::Wrapped { body } => body.is_end_stream(),
            ResponseBody::Empty => true,
        }
    }

    #[inline]
    fn size_hint(&self) -> SizeHint {
        match &self {
            ResponseBody::Full { body } => body.size_hint(),
            ResponseBody::Wrapped { body } => body.size_hint(),
            ResponseBody::Empty => SizeHint::with_exact(0),
        }
    }
}
