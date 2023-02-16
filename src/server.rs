use crate::command::Command;
use crate::mock::InnerMock;
use crate::request::Request;
use crate::response::{Body as ResponseBody, Chunked as ResponseChunked};
use crate::{Error, ErrorKind, Matcher, Mock};
use futures::stream::{self, StreamExt};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request as HyperRequest, Response, StatusCode};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct RemoteMock {
    pub(crate) inner: InnerMock,
}

impl RemoteMock {
    pub(crate) fn new(inner: InnerMock) -> Self {
        RemoteMock { inner }
    }

    async fn matches(&self, other: &mut Request) -> bool {
        self.method_matches(other)
            && self.path_matches(other)
            && self.headers_match(other)
            && self.body_matches(other).await
    }

    fn method_matches(&self, request: &Request) -> bool {
        self.inner.method.as_str() == request.method()
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.inner.path.matches_value(request.path_and_query())
    }

    fn headers_match(&self, request: &Request) -> bool {
        self.inner
            .headers
            .iter()
            .all(|&(ref field, ref expected)| expected.matches_values(&request.header(field)))
    }

    async fn body_matches(&self, request: &mut Request) -> bool {
        let body = request.read_body().await;
        let safe_body = &String::from_utf8_lossy(body);

        self.inner.body.matches_value(safe_body) || self.inner.body.matches_binary_value(body)
    }

    #[allow(clippy::missing_const_for_fn)]
    fn is_missing_hits(&self) -> bool {
        match (
            self.inner.expected_hits_at_least,
            self.inner.expected_hits_at_most,
        ) {
            (Some(_at_least), Some(at_most)) => self.inner.hits < at_most,
            (Some(at_least), None) => self.inner.hits < at_least,
            (None, Some(at_most)) => self.inner.hits < at_most,
            (None, None) => self.inner.hits < 1,
        }
    }
}

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) mocks: Vec<RemoteMock>,
    pub(crate) unmatched_requests: Vec<Request>,
}

impl State {
    fn new() -> Self {
        State {
            mocks: vec![],
            unmatched_requests: vec![],
        }
    }
}

///
/// One instance of the mock server.
///
/// Mockito uses a server pool to manage running servers. Once the pool reaches capacity,
/// new requests will have to wait for a free server. The size of the server pool
/// is set to 100.
///
/// Most of the times, you should initialize new servers with `Server::new`, which fetches
/// the next available instance from the pool:
///
/// ```
/// let mut server = mockito::Server::new();
/// ```
///
/// If for any reason you'd like to bypass the server pool, you can use `Server::new_with_port`:
///
/// ```
/// let mut server = mockito::Server::new_with_port(0);
/// ```
///
#[derive(Debug)]
pub struct Server {
    address: String,
    state: Arc<Mutex<State>>,
    sender: Sender<Command>,
    busy: bool,
}

impl Server {
    ///
    /// Fetches a new mock server from the server pool.
    ///
    /// This method will panic on failure.
    ///
    /// If for any reason you'd like to bypass the server pool, you can use `Server::new_with_port`:
    ///
    #[allow(clippy::new_ret_no_self)]
    #[track_caller]
    pub fn new() -> ServerGuard {
        Server::try_new().unwrap()
    }

    ///
    /// Same as `Server::new` but async.
    ///
    pub async fn new_async() -> ServerGuard {
        let server = Server::new_with_port_async(0).await;
        ServerGuard::new(server)
    }

    ///
    /// Same as `Server::new` but won't panic on failure.
    ///
    pub(crate) fn try_new() -> Result<ServerGuard, Error> {
        crate::RUNTIME.block_on(async { Server::try_new_async().await })
    }

    ///
    /// Same as `Server::try_new` but async.
    ///
    pub(crate) async fn try_new_async() -> Result<ServerGuard, Error> {
        let server = Server::try_new_with_port_async(0)
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;
        Ok(ServerGuard::new(server))
    }

    ///
    /// Starts a new server on a given port. If the port is set to `0`, a random available
    /// port will be assigned. Note that **this call bypasses the server pool**.
    ///
    /// This method will panic on failure.
    ///
    #[track_caller]
    pub fn new_with_port(port: u16) -> Server {
        Server::try_new_with_port(port).unwrap()
    }

    ///
    /// Same as `Server::try_new_with_port_async` but async.
    ///
    pub async fn new_with_port_async(port: u16) -> Server {
        Server::try_new_with_port_async(port).await.unwrap()
    }

    ///
    /// Same as `Server::new_with_port` but won't panic on failure.
    ///
    pub(crate) fn try_new_with_port(port: u16) -> Result<Server, Error> {
        crate::RUNTIME.block_on(async { Server::try_new_with_port_async(port).await })
    }

    ///
    /// Same as `Server::try_new_with_port` but async.
    ///
    pub(crate) async fn try_new_with_port_async(port: u16) -> Result<Server, Error> {
        let state = Arc::new(Mutex::new(State::new()));
        let address = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        let address = listener
            .local_addr()
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        let mutex = state.clone();
        let server = async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mutex = mutex.clone();

                tokio::spawn(async move {
                    Http::new()
                        .serve_connection(
                            stream,
                            service_fn(move |request: HyperRequest<Body>| {
                                handle_request(request, mutex.clone())
                            }),
                        )
                        .await
                        .unwrap();
                });
            }
        };

        thread::spawn(move || crate::RUNTIME.block_on(server));

        let (sender, receiver) = mpsc::channel(32);

        let mut server = Server {
            address: address.to_string(),
            state,
            sender,
            busy: true,
        };

        server.accept_commands(receiver).await;

        Ok(server)
    }

    ///
    /// Initializes a mock with the given HTTP `method` and `path`.
    ///
    /// The mock is enabled on the server only after calling the `Mock::create` method.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// let _m1 = s.mock("GET", "/");
    /// let _m2 = s.mock("POST", "/users");
    /// let _m3 = s.mock("DELETE", "/users?id=1");
    /// ```
    ///
    pub fn mock<P: Into<Matcher>>(&mut self, method: &str, path: P) -> Mock {
        Mock::new(self.sender.clone(), method, path)
    }

    ///
    /// The URL of the mock server (including the protocol).
    ///
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    ///
    /// The host and port of the mock server.
    /// Can be used with `std::net::TcpStream`.
    ///
    pub fn host_with_port(&self) -> String {
        self.address.clone()
    }

    ///
    /// Removes all the mocks stored on the server.
    ///
    pub fn reset(&mut self) {
        crate::RUNTIME.block_on(async { self.reset_async().await });
    }

    ///
    /// Same as `Server::reset` but async.
    ///
    pub async fn reset_async(&mut self) {
        let state = self.state.clone();
        let mut state = state.lock().await;
        state.mocks.clear();
        state.unmatched_requests.clear();
    }

    #[allow(dead_code)]
    pub(crate) fn busy(&self) -> bool {
        let state = self.state.clone();
        let locked = state.try_lock().is_err();
        let sender_busy = self.sender.try_send(Command::Noop).is_err();

        self.busy || locked || sender_busy
    }

    pub(crate) fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
    }

    async fn accept_commands(&mut self, mut receiver: Receiver<Command>) {
        let state = self.state.clone();
        tokio::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                let state = state.lock().await;
                Command::handle(cmd, state).await;
            }
        });

        log::debug!("Server is accepting commands");
    }
}

type GuardType = Server;

///
/// A handle around a pooled `Server` object which dereferences to `Server`.
///
pub struct ServerGuard {
    server: GuardType,
}

impl ServerGuard {
    pub(crate) fn new(mut server: GuardType) -> ServerGuard {
        server.set_busy(true);
        ServerGuard { server }
    }
}

impl Deref for ServerGuard {
    type Target = Server;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}

impl DerefMut for ServerGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.server
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.server.set_busy(false);
    }
}

async fn handle_request(
    hyper_request: HyperRequest<Body>,
    state: Arc<Mutex<State>>,
) -> Result<Response<Body>, Error> {
    let mut request = Request::new(hyper_request);
    log::debug!("Request received: {}", request.to_string().await);

    let mutex = state.clone();
    let mut state = mutex.lock().await;

    let mut mocks_stream = stream::iter(&mut state.mocks);
    let mut matching_mocks: Vec<&mut RemoteMock> = vec![];

    while let Some(mock) = mocks_stream.next().await {
        if mock.matches(&mut request).await {
            matching_mocks.push(mock);
        }
    }

    let maybe_missing_hits = matching_mocks.iter_mut().find(|m| m.is_missing_hits());

    let mock = match maybe_missing_hits {
        Some(m) => Some(m),
        None => matching_mocks.last_mut(),
    };

    if let Some(mock) = mock {
        log::debug!("Mock found");
        mock.inner.hits += 1;
        respond_with_mock(request, mock).await
    } else {
        log::debug!("Mock not found");
        state.unmatched_requests.push(request);
        respond_with_mock_not_found()
    }
}

async fn respond_with_mock(request: Request, mock: &RemoteMock) -> Result<Response<Body>, Error> {
    let status: StatusCode = mock.inner.response.status;
    let mut response = Response::builder().status(status);

    for (name, value) in mock.inner.response.headers.iter() {
        response = response.header(name, value);
    }

    let body = if request.method() != "HEAD" {
        match &mock.inner.response.body {
            ResponseBody::Bytes(bytes) => {
                if !request.has_header("content-length") {
                    response = response.header("content-length", bytes.len());
                }
                Body::from(bytes.clone())
            }
            ResponseBody::Fn(body_fn) => {
                let mut chunked = ResponseChunked::new();
                body_fn(&mut chunked)
                    .map_err(|_| Error::new(ErrorKind::ResponseBodyFailure))
                    .unwrap();
                chunked.finish();

                Body::wrap_stream(chunked)
            }
        }
    } else {
        Body::empty()
    };

    let response: Response<Body> = response
        .body(body)
        .map_err(|err| Error::new_with_context(ErrorKind::ResponseFailure, err))?;

    Ok(response)
}

fn respond_with_mock_not_found() -> Result<Response<Body>, Error> {
    let response: Response<Body> = Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .body(Body::empty())
        .map_err(|err| Error::new_with_context(ErrorKind::ResponseFailure, err))?;

    Ok(response)
}
