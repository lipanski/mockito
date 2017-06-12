use std::thread;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use serde_json;
use regex::Regex;
use {MockResponse, Matcher, SERVER_ADDRESS, Request};

#[derive(Serialize, Deserialize, Debug)]
struct RemoteMock {
    id: String,
    method: String,
    path: Matcher,
    headers: HashMap<String, Matcher>,
    response: MockResponse,
    hits: usize,
    expected_hits: usize,
}

impl RemoteMock {
    fn method_matches(&self, request: &Request) -> bool {
        self.method == request.method
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.path == request.path
    }

    fn headers_match(&self, request: &Request) -> bool {
        for (field, value) in self.headers.iter() {
            match request.headers.get(field) {
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
}

impl<'a> PartialEq<Request> for &'a mut RemoteMock {
    fn eq(&self, other: &Request) -> bool {
        self.method_matches(other)
            && self.path_matches(other)
            && self.headers_match(other)
    }
}

pub fn try_start() {
    if is_listening() { return }

    start()
}

fn start() {
    thread::spawn(move || {
        let mut mocks: Vec<RemoteMock> = vec!();
        let listener = TcpListener::bind(SERVER_ADDRESS).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let request = Request::from(&mut stream);
                    if request.is_ok() {
                        handle_request(&mut mocks, request, stream);
                    } else {
                        stream.write("HTTP/1.1 422 Unprocessable Entity\r\n\r\n".as_bytes()).unwrap();
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

fn handle_request(mut mocks: &mut Vec<RemoteMock>, request: Request, stream: TcpStream) {
    lazy_static! {
        static ref GET_MOCK_REGEX: Regex = Regex::new(r"^GET /mocks/(?P<mock_id>\w+)$").unwrap();
        static ref POST_MOCKS_REGEX: Regex = Regex::new(r"^POST /mocks$").unwrap();
        static ref DELETE_MOCKS_REGEX: Regex = Regex::new(r"^DELETE /mocks$").unwrap();
        static ref DELETE_MOCK_REGEX: Regex = Regex::new(r"^DELETE /mocks/(?P<mock_id>\w+)$").unwrap();
    }

    let request_line = format!("{} {}", request.method, request.path);

    if let Some(captures) = GET_MOCK_REGEX.captures(&request_line) {
        return handle_get_mock(mocks, captures["mock_id"].to_string(), stream);
    }

    if let Some(_) = POST_MOCKS_REGEX.captures(&request_line) {
        return handle_post_mock(mocks, request, stream);
    }

    if let Some(_) = DELETE_MOCKS_REGEX.captures(&request_line) {
        return handle_delete_mocks(mocks, stream);
    }

    if let Some(captures) = DELETE_MOCK_REGEX.captures(&request_line) {
        return handle_delete_mock(mocks, captures["mock_id"].to_string(), stream);
    }

    handle_match_mock(mocks, request, stream);
}

fn handle_get_mock(mocks: &mut Vec<RemoteMock>, mock_id: String, mut stream: TcpStream) {
    match mocks.iter().find(|mock| mock.id == mock_id) {
        Some(mock) => {
            let body = serde_json::to_string(mock).unwrap();
            let response = format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}", body.len(), body);
            stream.write(response.as_bytes()).unwrap();
        },
        None => {
            stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
        },
    }
}

fn handle_post_mock(mut mocks: &mut Vec<RemoteMock>, request: Request, mut stream: TcpStream) {
    match serde_json::from_slice::<RemoteMock>(&request.body) {
        Ok(mock) => {
            mocks.push(mock);
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
        },
        Err(err) => {
            let message = err.to_string();
            let response = format!("HTTP/1.1 422 Unprocessable Entity\r\ncontent-length: {}\r\n\r\n{}", message.len(), message);
            stream.write(response.as_bytes()).unwrap();
        }
    }
}

fn handle_delete_mocks(mut mocks: &mut Vec<RemoteMock>, mut stream: TcpStream) {
    mocks.clear();
    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
}

fn handle_delete_mock(mut mocks: &mut Vec<RemoteMock>, mock_id: String, mut stream: TcpStream) {
    match mocks.iter().position(|mock| mock.id == mock_id) {
        Some(pos) => {
            mocks.remove(pos);
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
        },
        None => {
            stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
        },
    };
}

fn handle_match_mock(mocks: &mut Vec<RemoteMock>, request: Request, mut stream: TcpStream) {
    match mocks.iter_mut().rev().find(|mock| mock == &request) {
        Some(mut mock) => {
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
            stream.write("HTTP/1.1 501 Not Implemented\r\n\r\n".as_bytes()).unwrap();
        }
    }
}
