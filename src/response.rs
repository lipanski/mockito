use crate::error::Error;
use crate::Request;
use futures::stream::Stream;
use hyper::StatusCode;
use std::fmt;
use std::io;
use std::sync::Arc;
use std::task::Poll;
use std::thread;
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Response {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

type BodyFnWithWriter = dyn Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static;
type BodyFnWithRequest = dyn Fn(&Request) -> Vec<u8> + Send + Sync + 'static;

#[derive(Clone)]
pub(crate) enum Body {
    Bytes(Vec<u8>),
    FnWithWriter(Arc<BodyFnWithWriter>),
    FnWithRequest(Arc<BodyFnWithRequest>),
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Body::Bytes(ref b) => b.fmt(f),
            Body::FnWithWriter(_) => f.write_str("<callback>"),
            Body::FnWithRequest(_) => f.write_str("<callback>"),
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Body::Bytes(ref a), Body::Bytes(ref b)) => a == b,
            (Body::FnWithWriter(ref a), Body::FnWithWriter(ref b)) => std::ptr::eq(
                a.as_ref() as *const BodyFnWithWriter as *const u8,
                b.as_ref() as *const BodyFnWithWriter as *const u8,
            ),
            (Body::FnWithRequest(ref a), Body::FnWithRequest(ref b)) => std::ptr::eq(
                a.as_ref() as *const BodyFnWithRequest as *const u8,
                b.as_ref() as *const BodyFnWithRequest as *const u8,
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

struct ChunkedStreamWriter {
    sender: mpsc::Sender<io::Result<Box<[u8]>>>,
}

impl io::Write for ChunkedStreamWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sender
            .blocking_send(Ok(buf.into()))
            .map_err(|_| io::ErrorKind::BrokenPipe)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) struct ChunkedStream {
    receiver: Option<mpsc::Receiver<io::Result<Box<[u8]>>>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ChunkedStream {
    pub fn new(body_fn: Arc<BodyFnWithWriter>) -> Result<Self, Error> {
        let (sender, receiver) = mpsc::channel(1);
        let join = thread::Builder::new()
            .name(format!("mockito::body_fn_{:p}", body_fn))
            .spawn(move || {
                let mut writer = ChunkedStreamWriter { sender };
                if let Err(e) = body_fn(&mut writer) {
                    let _ = writer.sender.blocking_send(Err(e));
                }
            })
            .map_err(|e| Error::new_with_context(crate::ErrorKind::ResponseFailure, e))?;
        Ok(Self {
            receiver: Some(receiver),
            thread: Some(join),
        })
    }
}

impl Drop for ChunkedStream {
    fn drop(&mut self) {
        // must close the channel first
        let _ = self.receiver.take();
        let _ = self.thread.take().map(|t| t.join());
    }
}

impl Stream for ChunkedStream {
    type Item = io::Result<Box<[u8]>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.receiver
            .as_mut()
            .map(move |r| r.poll_recv(cx))
            .unwrap_or(Poll::Ready(None))
    }
}
