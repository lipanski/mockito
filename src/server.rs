use std::thread;
use std::io::Write;
use std::fmt::Display;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::Mutex;
use std::sync::mpsc;
use {SERVER_ADDRESS_INTERNAL, Request, Mock};
use response::Body;

impl Mock {
    fn method_matches(&self, request: &Request) -> bool {
        self.method == request.method
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.path.matches_value(&request.path)
    }

    fn query_matches(&self, request: &Request) -> bool {
        self.query.matches_value(&request.query)
    }

    fn headers_match(&self, request: &Request) -> bool {
        self.headers.iter().all(|&(ref field, ref expected)| {
            expected.matches_values(&request.find_header_values(field))
        })
    }

    fn body_matches(&self, request: &Request) -> bool {
        self.body.matches_value(&String::from_utf8_lossy(&request.body))
    }
}

impl<'a> PartialEq<Request> for &'a mut Mock {
    fn eq(&self, other: &Request) -> bool {
        self.method_matches(other)
            && self.path_matches(other)
            && self.query_matches(other)
            && self.headers_match(other)
            && self.body_matches(other)
    }
}

pub struct State {
    pub listening_addr: Option<SocketAddr>,
    pub mocks: Vec<Mock>,
    pub unmatched_requests: Vec<Request>,
}

impl State {
    fn new() -> Self {
        Self {
            listening_addr: None,
            mocks: Vec::new(),
            unmatched_requests: Vec::new(),
        }
    }
}

lazy_static! {
    pub static ref STATE: Mutex<State> = Mutex::new(State::new());
}

/// Address and port of the local server.
/// Can be used with `std::net::TcpStream`.
///
/// The server will be started if necessary.
pub fn address() -> SocketAddr {
    try_start();

    let state = STATE.lock().map(|state| state.listening_addr);
    state.expect("state lock").expect("server should be listening")
}

/// A local `http://…` URL of the server.
///
/// The server will be started if necessary.
pub fn url() -> String {
    format!("http://{}", address())
}

pub fn try_start() {
    let mut state = STATE.lock().unwrap();

    if state.listening_addr.is_some() {
        return
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let res = TcpListener::bind(SERVER_ADDRESS_INTERNAL)
        .or_else(|err| {
            warn!("{}", err);
            TcpListener::bind("127.0.0.1:0")
        });
        let (listener, addr) = match res {
            Ok(listener) => {
                let addr = listener.local_addr().unwrap();
                tx.send(Some(addr)).unwrap();
                (listener, addr)
            },
            Err(err) => {
                error!("{}", err);
                tx.send(None).unwrap();
                return;
            },
        };

        debug!("Server is listening at {}", addr);
        for stream in listener.incoming() {
            if let Ok(stream) = stream{
                let request = Request::from(&stream);
                debug!("Request received: {}", request);
                if request.is_ok() {
                    handle_request(request, stream);
                } else {
                    let message = request.error().map_or("Could not parse the request.", |err| err.as_str());
                    debug!("Could not parse request because: {}", message);
                    respond_with_error(stream, request.version, message);
                }
            } else {
                debug!("Could not read from stream");
            }
        }
    });

    state.listening_addr = rx.recv().ok().and_then(|addr| addr);
}

fn handle_request(request: Request, stream: TcpStream) {
    handle_match_mock(request, stream);
}

fn handle_match_mock(request: Request, stream: TcpStream) {
    let found;

    let mut state = STATE.lock().unwrap();

    if let Some(mock) = state.mocks.iter_mut().rev().find(|mock| mock == &request) {
        debug!("Mock found");
        found = true;
        mock.hits += 1;
        respond_with_mock(stream, request.version, mock, request.is_head());
    } else {
        debug!("Mock not found");
        found = false;
        respond_with_mock_not_found(stream, request.version);
    }

    if !found { state.unmatched_requests.push(request); }
}

fn respond(
    stream: TcpStream,
    version: (u8, u8),
    status: impl Display,
    headers: Option<&Vec<(String, String)>>,
    body: Option<&str>
) {
    let body = body.map(|s| Body::Bytes(s.as_bytes().to_owned()));
    respond_bytes(stream, version, status, headers, body.as_ref())
}

fn respond_bytes(
    mut stream: TcpStream,
    version: (u8, u8),
    status: impl Display,
    headers: Option<&Vec<(String, String)>>,
    body: Option<&Body>
) {
    let mut response = Vec::from(format!("HTTP/{}.{} {}\r\n", version.0, version.1, status));
    let mut has_content_length_header = false;

    if let Some(headers) = headers {
        for &(ref key, ref value) in headers {
            response.extend(key.as_bytes());
            response.extend(b": ");
            response.extend(value.as_bytes());
            response.extend(b"\r\n");
        }

        has_content_length_header = headers.iter().any(|(key, _)| key == "content-length");
    }

    let mut buffer;
    let body = match body {
        Some(Body::Bytes(bytes)) => Some(&bytes[..]),
        Some(Body::Fn(cb)) => {
            // we don't implement transfer-encoding: chunked, so need to buffer
            buffer = Vec::new();
            let _ = cb(&mut buffer);
            Some(&buffer[..])
        },
        None => None,
    };
    if let Some(bytes) = body {
        if !has_content_length_header {
            response.extend(format!("content-length: {}\r\n", bytes.len()).as_bytes());
        }
    }
    response.extend(b"\r\n");
    let _ = stream.write(&response);
    if let Some(bytes) = body {
        let _ = stream.write(bytes);
    }
    let _ = stream.flush();
}

fn respond_with_mock(stream: TcpStream, version: (u8, u8), mock: &Mock, skip_body: bool) {
    let body =
        if skip_body {
            None
        } else {
            Some(&mock.response.body)
        };

    respond_bytes(stream, version, &mock.response.status, Some(&mock.response.headers), body);
}

fn respond_with_mock_not_found(stream: TcpStream, version: (u8, u8)) {
    respond(stream, version, "501 Mock Not Found", None, None);
}

fn respond_with_error(stream: TcpStream, version: (u8, u8), message: &str) {
    respond(stream, version, "422 Mock Error", None, Some(message));
}
