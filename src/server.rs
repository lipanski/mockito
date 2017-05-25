use std::thread;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use serde_json;
use {Mock, SERVER_ADDRESS, Request};

pub fn try_start() {
    if is_listening() { return }

    start()
}

fn start() {
    thread::spawn(move || {
        let mut mocks: Vec<Mock> = vec!();
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

fn handle_request(mut mocks: &mut Vec<Mock>, request: Request, stream: TcpStream) {
    match (&*request.method, &*request.path) {
        ("POST", "/mocks") => handle_create_mock(mocks, request, stream),
        ("DELETE", "/mocks") => handle_delete_mock(mocks, request, stream),
        _ => handle_match_mock(mocks, request, stream),
    }
}

fn handle_create_mock(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    match serde_json::from_slice(&request.body) {
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

fn handle_delete_mock(mut mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    match request.headers.iter().find(|&(ref field, _)| { field.to_lowercase() == "x-mock-id" }) {
        // Remove the element with x-mock-id
        Some((_, value)) => {
            match mocks.iter().position(|mock| &mock.id == value) {
                Some(pos) => { mocks.remove(pos); },
                None => {},
            };
        },
        // Remove all elements
        None => { mocks.clear(); }
    }

    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
}

fn handle_match_mock(mocks: &mut Vec<Mock>, request: Request, mut stream: TcpStream) {
    match mocks.iter().rev().find(|mock| mock.matches(&request)) {
        Some(mock) => {
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
