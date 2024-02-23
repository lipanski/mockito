use crate::mock::InnerMock;
use crate::request::Request;
use crate::response::{Body as ResponseBody, ChunkedStream};
use crate::ServerGuard;
use crate::{Error, ErrorKind, Matcher, Mock};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request as HyperRequest, Response, StatusCode};
use std::default::Default;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::ops::Drop;
use std::str::FromStr;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::task::{spawn_local, LocalSet};

#[derive(Clone, Debug)]
pub(crate) struct RemoteMock {
    pub(crate) inner: InnerMock,
}

impl RemoteMock {
    pub(crate) fn new(inner: InnerMock) -> Self {
        RemoteMock { inner }
    }

    fn matches(&self, other: &mut Request) -> bool {
        self.method_matches(other)
            && self.path_matches(other)
            && self.headers_match(other)
            && self.body_matches(other)
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
            .all(|(field, expected)| expected.matches_values(&request.header(field)))
    }

    fn body_matches(&self, request: &mut Request) -> bool {
        let body = request.body().unwrap();
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

    pub(crate) fn get_mock_hits(&self, mock_id: String) -> Option<usize> {
        self.mocks
            .iter()
            .find(|remote_mock| remote_mock.inner.id == mock_id)
            .map(|remote_mock| remote_mock.inner.hits)
    }

    pub(crate) fn remove_mock(&mut self, mock_id: String) -> bool {
        if let Some(pos) = self
            .mocks
            .iter()
            .position(|remote_mock| remote_mock.inner.id == mock_id)
        {
            self.mocks.remove(pos);
            return true;
        }

        false
    }

    pub(crate) fn get_last_unmatched_request(&self) -> Option<String> {
        self.unmatched_requests.last().map(|req| req.formatted())
    }
}

///
/// Options to configure a mock server. Provides a default implementation.
///
/// ```
/// let opts = mockito::ServerOpts { port: 1234, ..Default::default() };
/// ```
///
pub struct ServerOpts {
    /// The server host (defaults to 127.0.0.1)
    pub host: &'static str,
    /// The server port (defaults to a randomly assigned free port)
    pub port: u16,
    /// Automatically call `assert()` before dropping a mock (defaults to false)
    pub assert_on_drop: bool,
}

impl ServerOpts {
    pub(crate) fn address(&self) -> SocketAddr {
        let ip = IpAddr::from_str(self.host).unwrap();
        SocketAddr::from((ip, self.port))
    }
}

impl Default for ServerOpts {
    fn default() -> Self {
        let host = "127.0.0.1";
        let port = 0;
        let assert_on_drop = false;

        ServerOpts {
            host,
            port,
            assert_on_drop,
        }
    }
}

///
/// One instance of the mock server.
///
/// Mockito uses a server pool to manage running servers. Once the pool reaches capacity,
/// new requests will have to wait for a free server. The size of the server pool
/// is set to 50.
///
/// Most of the times, you should initialize new servers with `Server::new`, which fetches
/// the next available instance from the pool:
///
/// ```
/// let mut server = mockito::Server::new();
/// ```
///
/// If you'd like to bypass the server pool or configure the server in a different way
/// (by setting a custom host and port or enabling auto-asserts), you can use `Server::new_with_opts`:
///
/// ```
/// let opts = mockito::ServerOpts { port: 0, ..Default::default() };
/// let server_with_port = mockito::Server::new_with_opts(opts);
///
/// let opts = mockito::ServerOpts { host: "0.0.0.0", ..Default::default() };
/// let server_with_host = mockito::Server::new_with_opts(opts);
///
/// let opts = mockito::ServerOpts { assert_on_drop: true, ..Default::default() };
/// let server_with_auto_assert = mockito::Server::new_with_opts(opts);
/// ```
///
#[derive(Debug)]
pub struct Server {
    address: SocketAddr,
    state: Arc<RwLock<State>>,
    assert_on_drop: bool,
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
        Server::try_new_async().await.unwrap()
    }

    ///
    /// Same as `Server::new` but won't panic on failure.
    ///
    #[track_caller]
    pub(crate) fn try_new() -> Result<ServerGuard, Error> {
        runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Cannot build local tokio runtime")
            .block_on(async { Server::try_new_async().await })
    }

    ///
    /// Same as `Server::try_new` but async.
    ///
    pub(crate) async fn try_new_async() -> Result<ServerGuard, Error> {
        let server = crate::server_pool::SERVER_POOL
            .get_async()
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        Ok(server)
    }

    ///
    /// **DEPRECATED:** Use `Server::new_with_opts` instead.
    ///
    #[deprecated(since = "1.3.0", note = "Use `Server::new_with_opts` instead")]
    #[track_caller]
    pub fn new_with_port(port: u16) -> Server {
        let opts = ServerOpts {
            port,
            ..Default::default()
        };
        Server::try_new_with_opts(opts).unwrap()
    }

    ///
    /// Starts a new server with the given options. Note that **this call bypasses the server pool**.
    ///
    /// This method will panic on failure.
    ///
    #[track_caller]
    pub fn new_with_opts(opts: ServerOpts) -> Server {
        Server::try_new_with_opts(opts).unwrap()
    }

    ///
    /// **DEPRECATED:** Use `Server::new_with_opts_async` instead.
    ///
    #[deprecated(since = "1.3.0", note = "Use `Server::new_with_opts_async` instead")]
    pub async fn new_with_port_async(port: u16) -> Server {
        let opts = ServerOpts {
            port,
            ..Default::default()
        };
        Server::try_new_with_opts_async(opts).await.unwrap()
    }

    ///
    /// Same as `Server::new_with_opts` but async.
    ///
    pub async fn new_with_opts_async(opts: ServerOpts) -> Server {
        Server::try_new_with_opts_async(opts).await.unwrap()
    }

    ///
    /// Same as `Server::new_with_opts` but won't panic on failure.
    ///
    #[track_caller]
    pub(crate) fn try_new_with_opts(opts: ServerOpts) -> Result<Server, Error> {
        let state = Arc::new(RwLock::new(State::new()));
        let address = opts.address();
        let assert_on_drop = opts.assert_on_drop;
        let (address_sender, address_receiver) = mpsc::channel::<SocketAddr>();
        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Cannot build local tokio runtime");

        let state_clone = state.clone();
        thread::spawn(move || {
            let server = Server::bind_server(address, address_sender, state_clone);
            LocalSet::new().block_on(&runtime, server).unwrap();
        });

        let address = address_receiver
            .recv()
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        let server = Server {
            address,
            state,
            assert_on_drop,
        };

        Ok(server)
    }

    ///
    /// Same as `Server::try_new_with_opts` but async.
    ///
    pub(crate) async fn try_new_with_opts_async(opts: ServerOpts) -> Result<Server, Error> {
        let state = Arc::new(RwLock::new(State::new()));
        let address = opts.address();
        let assert_on_drop = opts.assert_on_drop;
        let (address_sender, address_receiver) = mpsc::channel::<SocketAddr>();
        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Cannot build local tokio runtime");

        let state_clone = state.clone();
        thread::spawn(move || {
            let server = Server::bind_server(address, address_sender, state_clone);
            LocalSet::new().block_on(&runtime, server).unwrap();
        });

        let address = address_receiver
            .recv()
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        let server = Server {
            address,
            state,
            assert_on_drop,
        };

        Ok(server)
    }

    async fn bind_server(
        address: SocketAddr,
        address_sender: mpsc::Sender<SocketAddr>,
        state: Arc<RwLock<State>>,
    ) -> Result<(), Error> {
        let listener = TcpListener::bind(address)
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        let address = listener
            .local_addr()
            .map_err(|err| Error::new_with_context(ErrorKind::ServerFailure, err))?;

        address_sender.send(address).unwrap();

        while let Ok((stream, _)) = listener.accept().await {
            let mutex = state.clone();

            spawn_local(async move {
                let _ = Http::new()
                    .serve_connection(
                        stream,
                        service_fn(move |request: HyperRequest<Body>| {
                            handle_request(request, mutex.clone())
                        }),
                    )
                    .await;
            });
        }

        Ok(())
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
        Mock::new(self.state.clone(), method, path, self.assert_on_drop)
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
        self.address.to_string()
    }

    ///
    /// The raw address of the mock server.
    ///
    pub fn socket_address(&self) -> SocketAddr {
        self.address
    }

    ///
    /// Removes all the mocks stored on the server.
    ///
    pub fn reset(&mut self) {
        let state = self.state.clone();
        let mut state = state.write().unwrap();
        state.mocks.clear();
        state.unmatched_requests.clear();
    }

    ///
    /// **DEPRECATED:** Use `Server::reset` instead. The implementation is not async any more.
    ///
    #[deprecated(since = "1.0.1", note = "Use `Server::reset` instead")]
    pub async fn reset_async(&mut self) {
        let state = self.state.clone();
        let mut state = state.write().unwrap();
        state.mocks.clear();
        state.unmatched_requests.clear();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.reset();
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("server {}", self.host_with_port()))
    }
}

async fn handle_request(
    hyper_request: HyperRequest<Body>,
    state: Arc<RwLock<State>>,
) -> Result<Response<Body>, Error> {
    let mut request = Request::new(hyper_request);
    request.read_body().await;
    log::debug!("Request received: {}", request.formatted());

    let mutex = state.clone();
    let mut state = mutex.write().unwrap();
    let mut matching_mocks: Vec<&mut RemoteMock> = vec![];

    for mock in state.mocks.iter_mut() {
        if mock.matches(&mut request) {
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
        respond_with_mock(request, mock)
    } else {
        log::debug!("Mock not found");
        state.unmatched_requests.push(request);
        respond_with_mock_not_found()
    }
}

fn respond_with_mock(request: Request, mock: &RemoteMock) -> Result<Response<Body>, Error> {
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
            ResponseBody::FnWithWriter(body_fn) => {
                let stream = ChunkedStream::new(Arc::clone(body_fn))?;
                Body::wrap_stream(stream)
            }
            ResponseBody::FnWithRequest(body_fn) => {
                let bytes = body_fn(&request);
                Body::from(bytes)
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
