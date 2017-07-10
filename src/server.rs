use std::thread;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use {Matcher, SERVER_ADDRESS, Request, Mock};

impl Mock {
    fn method_matches(&self, request: &Request) -> bool {
        self.method == request.method
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.path == request.path
    }

    fn headers_match(&self, request: &Request) -> bool {
        for &(ref field, ref value) in &self.headers {
            match request.find_header(field) {
                Some(request_header_value) => {
                    if value == request_header_value { continue }

                    return false
                },
                None => {
                    if value == &Matcher::Missing { continue }

                    return false
                },
            }
        }

        true
    }

    fn body_matches(&self, request: &Request) -> bool {
        self.body == String::from_utf8_lossy(&request.body).into_owned()
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
    pub mocks: Vec<Mock>,
    pub unmatched_requests: Vec<Request>,
}

impl Default for State {
    fn default() -> Self {
        State {
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
    thread::spawn(move || {
        let listener = TcpListener::bind(SERVER_ADDRESS).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let request = Request::from(&mut stream);
                    if request.is_ok() {
                        handle_request(request, stream);
                    } else {
                        let body = request.error().map_or("Could not parse the request.", |err| err.as_str());
                        let response = format!("HTTP/1.1 422 Unprocessable Entity\r\ncontent-length: {}\r\n\r\n{}", body.len(), body);
                        stream.write(response.as_bytes()).unwrap();
                    }
                },
                Err(_) => {},
            }
        }
    });

    while !is_listening() {}
}

fn is_listening() -> bool {
    TcpStream::connect(SERVER_ADDRESS).is_ok()
}

fn handle_request(request: Request, stream: TcpStream) {
    handle_match_mock(request, stream);
}

fn handle_match_mock(request: Request, mut stream: TcpStream) {
    let found;

    let state_mutex = STATE.clone();
    let mut state = state_mutex.lock().unwrap();

    match state.mocks.iter_mut().rev().find(|mock| mock == &request) {
        Some(mut mock) => {
            found = true;

            mock.hits = mock.hits + 1;

            let mut headers = String::new();
            for &(ref key, ref value) in &mock.response.headers {
                headers.push_str(key);
                headers.push_str(": ");
                headers.push_str(value);
                headers.push_str("\r\n");
            }

            let ref body = mock.response.body;

            let response = format!("HTTP/1.1 {}\r\ncontent-length: {}\r\n{}\r\n{}", mock.response.status, body.len(), headers, body);
            stream.write(response.as_bytes()).unwrap();
        },
        None => {
            found = false;
            stream.write("HTTP/1.1 501 Not Implemented\r\n\r\n".as_bytes()).unwrap();
        }
    }

    if !found { state.unmatched_requests.push(request); }
}
