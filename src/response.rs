use core::task::Poll;
use futures::stream::Stream;
use hyper::StatusCode;
use std::fmt;
use std::io;
use std::mem;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Response {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

type BodyFn = dyn Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static;

#[derive(Clone)]
pub(crate) enum Body {
    Bytes(Vec<u8>),
    Fn(Arc<BodyFn>),
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Body::Bytes(ref b) => b.fmt(f),
            Body::Fn(_) => f.write_str("<callback>"),
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Body::Bytes(ref a), Body::Bytes(ref b)) => a == b,
            (Body::Fn(ref a), Body::Fn(ref b)) => std::ptr::eq(
                a.as_ref() as *const BodyFn as *const u8,
                b.as_ref() as *const BodyFn as *const u8,
            ),
            _ => false,
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            status: StatusCode::OK,
            headers: vec![("connection".into(), "close".into())],
            body: Body::Bytes(Vec::new()),
        }
    }
}

pub(crate) struct Chunked {
    buffer: Vec<u8>,
    finished: bool,
}

impl Chunked {
    pub fn new() -> Self {
        Self {
            buffer: vec![],
            finished: false,
        }
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }
}

impl Stream for Chunked {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if !self.buffer.is_empty() {
            let data = mem::take(&mut self.buffer);
            Poll::Ready(Some(Ok(data)))
        } else if !self.finished {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}

impl io::Write for Chunked {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.append(&mut buf.to_vec());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.finished = true;
        Ok(())
    }
}
