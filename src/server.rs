use Mock;

use std::net::{TcpStream, TcpListener};
use std::io::{Write, Read, BufReader, BufRead};
use std::thread;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering, ATOMIC_USIZE_INIT, ATOMIC_BOOL_INIT};
use std::iter::Iterator;
use std::collections::HashMap;

pub static PORT: AtomicUsize = ATOMIC_USIZE_INIT;
pub static SERVER_THREAD_SPAWNED: AtomicBool = ATOMIC_BOOL_INIT;
pub static REQUEST_SERVER_STOP: AtomicBool = ATOMIC_BOOL_INIT;
pub static STOP_REQUEST: &'static [u8] = b"STOP";

pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String
}

impl Request {
    pub fn is_internal(&self) -> bool {
        self.path == "/mockito"
    }
}

pub fn listen() {
    if !is_listening() { start(None) };
}

pub fn start(port: Option<u16>) {
    let requested_port = port.unwrap_or(0);
    let listener = TcpListener::bind(&*format!("127.0.0.1:{}", requested_port)).unwrap();
    let assigned_port = listener.local_addr().unwrap().port();

    set_port(assigned_port);

    thread::spawn(move || {
        let mut mocks: Vec<Mock> = Vec::new();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_request(stream, &mut mocks),
                Err(e)     => println!("Error: {}", e)
            }
        }

        drop(listener);
    });

    while !is_listening() {}
}

pub fn new(mock: String) {
    let mut stream = TcpStream::connect(&*host()).unwrap();

    stream.write_all(b"POST /mockito HTTP/1.1\n\n").unwrap_or(());
    stream.write_all(mock.as_bytes()).unwrap_or(());
}

pub fn reset() {
    let mut stream = TcpStream::connect(&*host()).unwrap();

    stream.write_all(b"DELETE /mockito HTTP/1.1\n\n").unwrap_or(());
}

pub fn is_listening() -> bool {
    TcpStream::connect(&*host()).is_ok()
}

pub fn port() -> u16 {
    PORT.load(Ordering::SeqCst) as u16
}

fn set_port(port: u16) {
    PORT.store(port as usize, Ordering::SeqCst);
}

pub fn host() -> String {
    format!("127.0.0.1:{}", port())
}

pub fn host_with_protocol() -> String {
    format!("http://127.0.0.1:{}", port())
}

fn handle_request(mut stream: TcpStream, mocks: &mut Vec<Mock>) {
    match parse(&mut stream) {
        Some(ref request) if request.is_internal() => handle_command(&mut stream, request, mocks),
        Some(ref request) => handle_mock(&mut stream, request, mocks),
        None => { stream.write_all(b"HTTP/1.1 400 Bad Request\n\n").unwrap_or(()); }
    }
}

fn handle_command(stream: &mut TcpStream, request: &Request, mocks: &mut Vec<Mock>) {
    match (&*request.method, &*request.path) {
        ("POST", "/mockito") => {
            match Mock::from(&request) {
                Some(mock) => {
                    mocks.insert(0, mock); // Last insert should match first
                    stream.write_all(b"HTTP/1.1 200 OK\n\n").unwrap_or(());
                },
                _ => {
                    stream.write_all(b"HTTP/1.1 422 Unprocessable Entity\n\nX-Mock-Method and/or X-Mock-Path headers missing.").unwrap_or(());

                }
            }
        },
        ("DELETE", "/mockito") => {
            mocks.clear();
            stream.write_all(b"HTTP/1.1 200 OK\n\n").unwrap_or(());
        },
        _ => { stream.write_all(b"HTTP/1.1 404 Not Found\n\n").unwrap_or(()); }
    }
}

fn handle_mock(stream: &mut TcpStream, request: &Request, mocks: &mut Vec<Mock>) {
    match mocks.iter().find(|mock| mock.matches(request)) {
        Some(mock) => {
            stream.write_all(mock.response().as_bytes()).unwrap_or(());
            return
        },
        None => { stream.write_all(b"HTTP/1.1 501 Not Implemented\n\n").unwrap_or(()); }
    }
}

fn parse(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() { return None }

    // Parse request line
    let (method, path) = match request_line.split_whitespace().collect::<Vec<&str>>() {
        ref elements if elements.len() == 2 || elements.len() == 3 => (elements[0], elements[1]),
        _ => return None
    };

    // Parse headers
    let mut headers = HashMap::new();
    loop {
        let mut header_line = String::new();
        match reader.read_line(&mut header_line) {
            Ok(_) if header_line.trim().len() > 0 => {
                let parts: Vec<&str> = header_line.splitn(2, ":").collect();

                if parts.len() != 2 { return None }

                let field = parts[0].trim().to_lowercase().to_string();
                let value = parts[1].trim().to_string();

                headers.insert(field, value);
            },
            Ok(_) => break,
            Err(_) => return None
        }
    }

    // TODO: Ignore body if request is internal

    // Parse body
    let default_content_length = "0".to_string();
    let content_length = headers.get("content-length").unwrap_or(&default_content_length).clone();
    let length = content_length.parse::<u64>().unwrap_or(0);

    let mut body = String::new();
    reader.take(length).read_to_string(&mut body).unwrap_or(0);

    Some(Request {
        method: method.to_string(),
        path: path.to_string(),
        headers: headers,
        body: body
    })
}
