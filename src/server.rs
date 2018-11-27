use std::thread;
use std::io::Write;
use std::fmt::Display;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use {SERVER_ADDRESS, Request, Mock};

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
        self.body.matches_value(&String::from_utf8_lossy(&request.body))
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
    pub is_listening: bool,
    pub mocks: Vec<Mock>,
    pub unmatched_requests: Vec<Request>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_listening: is_listening(),
            mocks: Vec::new(),
            unmatched_requests: Vec::new(),
        }
    }
}

lazy_static! {
    pub static ref STATE: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
}

pub fn try_start() {
    if is_listening() { return }

    start()
}

fn start() {
    let state_mutex = STATE.clone();
    let mut state = state_mutex.lock().unwrap();

    if state.is_listening { return }

    thread::spawn(move || {
        let listener = TcpListener::bind(SERVER_ADDRESS).unwrap();
        debug!("Server is listening");
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream{
                let request = Request::from(&mut stream);
                debug!("Request received: {}", request);
                if request.is_ok() {
                    handle_request(request, stream);
                } else {
                    let message = request.error().map_or("Could not parse the request.", |err| err.as_str());
                    debug!("Could not parse request because: {}", message);
                    respond_with_error(stream, message);
                }
            } else {
                debug!("Could not read from stream");
            }
        }
    });

    while !is_listening() {}

    state.is_listening = true;
}

fn is_listening() -> bool {
    TcpStream::connect(SERVER_ADDRESS).is_ok()
}

fn handle_request(request: Request, stream: TcpStream) {
    handle_match_mock(request, stream);
}

fn handle_match_mock(request: Request, stream: TcpStream) {
    let found;

    let state_mutex = STATE.clone();
    let mut state = state_mutex.lock().unwrap();

    if let Some(mock) = state.mocks.iter_mut().rev().find(|mock| mock == &request) {
        debug!("Mock found");
        found = true;
        mock.hits += 1;
        respond_with_mock(stream, mock);
    } else {
        debug!("Mock not found");
        found = false;
        respond_with_mock_not_found(stream);
    }

    if !found { state.unmatched_requests.push(request); }
}

fn respond<S: Display>(stream: TcpStream, status: S, headers: Option<&str>, body: Option<&str>) {
    respond_bytes(stream, status, headers, body.map(|s| s.as_bytes()))
}

fn respond_bytes<S: Display>(mut stream: TcpStream, status: S, headers: Option<&str>, body: Option<&[u8]>) {
    let mut response = Vec::from(format!("HTTP/1.1 {}\r\n", status));

    if let Some(headers) = headers {
        response.extend(headers.as_bytes());
    }

    if let Some(body) = body {
        response.extend(format!("content-length: {}\r\n\r\n", body.len()).as_bytes());
        response.extend(body);
    } else {
        response.extend(b"\r\n");
    }

    let _ = stream.write(&response);
    let _ = stream.flush();
}

fn respond_with_mock(stream: TcpStream, mock: &Mock) {
    let mut headers = String::new();
    for &(ref key, ref value) in &mock.response.headers {
        headers.push_str(key);
        headers.push_str(": ");
        headers.push_str(value);
        headers.push_str("\r\n");
    }

    respond_bytes(stream, &mock.response.status, Some(&headers), Some(&mock.response.body));
}

fn respond_with_mock_not_found(stream: TcpStream) {
    respond(stream, "501 Mock Not Found", None, None);
}

fn respond_with_error(stream: TcpStream, message: &str) {
    respond(stream, "422 Mock Error", None, Some(message));
}
