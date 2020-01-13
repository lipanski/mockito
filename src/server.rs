use crate::response::{Body, Chunked};
use crate::{Mock, Request, SERVER_ADDRESS_INTERNAL};
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

impl Mock {
    fn method_matches(&self, request: &Request) -> bool {
        self.method == request.method
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.path.matches_value(&request.path)
    }

    fn headers_match(&self, request: &Request) -> bool {
        self.headers.iter().all(|&(ref field, ref expected)| {
            expected.matches_values(&request.find_header_values(field))
        })
    }

    fn body_matches(&self, request: &Request) -> bool {
        self.body
            .matches_value(&String::from_utf8_lossy(&request.body))
    }

    fn is_missing_hits(&self) -> bool {
        match (self.expected_hits_at_least, self.expected_hits_at_most) {
            (Some(_at_least), Some(at_most)) => self.hits < at_most,
            (Some(at_least), None) => self.hits < at_least,
            (None, Some(at_most)) => self.hits < at_most,
            (None, None) => self.hits < 1,
        }
    }
}

impl<'a> PartialEq<Request> for &'a mut Mock {
    fn eq(&self, other: &Request) -> bool {
        self.method_matches(other)
            && self.path_matches(other)
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
    state
        .expect("state lock")
        .expect("server should be listening")
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
        return;
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let res = TcpListener::bind(SERVER_ADDRESS_INTERNAL).or_else(|err| {
            warn!("{}", err);
            TcpListener::bind("127.0.0.1:0")
        });
        let (listener, addr) = match res {
            Ok(listener) => {
                let addr = listener.local_addr().unwrap();
                tx.send(Some(addr)).unwrap();
                (listener, addr)
            }
            Err(err) => {
                error!("{}", err);
                tx.send(None).unwrap();
                return;
            }
        };

        debug!("Server is listening at {}", addr);
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let request = Request::from(&stream);
                debug!("Request received: {}", request);
                if request.is_ok() {
                    handle_request(request, stream);
                } else {
                    let message = request
                        .error()
                        .map_or("Could not parse the request.", |err| err.as_str());
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

    let mut mocks_matched = state
        .mocks
        .iter_mut()
        .rev()
        .filter(|mock| mock == &request)
        .collect::<Vec<_>>();

    let mock = if let Some(mock) = mocks_matched
        .iter_mut()
        .rev()
        .find(|mock| mock.is_missing_hits())
    {
        Some(mock)
    } else {
        mocks_matched.last_mut()
    };

    if let Some(mock) = mock {
        debug!("Mock found");
        found = true;
        mock.hits += 1;
        respond_with_mock(stream, request.version, mock, request.is_head());
    } else {
        debug!("Mock not found");
        found = false;
        respond_with_mock_not_found(stream, request.version);
    }

    if !found {
        state.unmatched_requests.push(request);
    }
}

fn respond(
    stream: TcpStream,
    version: (u8, u8),
    status: impl Display,
    headers: Option<&Vec<(String, String)>>,
    body: Option<&str>,
) {
    let body = body.map(|s| Body::Bytes(s.as_bytes().to_owned()));
    if let Err(e) = respond_bytes(stream, version, status, headers, body.as_ref()) {
        eprintln!("warning: Mock response write error: {}", e);
    }
}

fn respond_bytes(
    mut stream: TcpStream,
    version: (u8, u8),
    status: impl Display,
    headers: Option<&Vec<(String, String)>>,
    body: Option<&Body>,
) -> io::Result<()> {
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

    match body {
        Some(Body::Bytes(bytes)) => {
            if !has_content_length_header {
                response.extend(format!("content-length: {}\r\n", bytes.len()).as_bytes());
            }
        }
        Some(Body::Fn(_)) => {
            response.extend(b"transfer-encoding: chunked\r\n");
        }
        None => {}
    };
    response.extend(b"\r\n");
    stream.write_all(&response)?;
    match body {
        Some(Body::Bytes(bytes)) => {
            stream.write_all(bytes)?;
        }
        Some(Body::Fn(cb)) => {
            let mut chunked = Chunked::new(&mut stream);
            cb(&mut chunked)?;
            chunked.finish()?;
        }
        None => {}
    };
    stream.flush()
}

fn respond_with_mock(stream: TcpStream, version: (u8, u8), mock: &Mock, skip_body: bool) {
    let body = if skip_body {
        None
    } else {
        Some(&mock.response.body)
    };

    if let Err(e) = respond_bytes(
        stream,
        version,
        &mock.response.status,
        Some(&mock.response.headers),
        body,
    ) {
        eprintln!("warning: Mock response write error: {}", e);
    }
}

fn respond_with_mock_not_found(stream: TcpStream, version: (u8, u8)) {
    respond(stream, version, "501 Mock Not Found", None, None);
}

fn respond_with_error(stream: TcpStream, version: (u8, u8), message: &str) {
    respond(stream, version, "422 Mock Error", None, Some(message));
}
