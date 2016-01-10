use Mock;
use server;

use std::net::TcpStream;
use std::io::Write;

pub fn new(mock: &Mock) {
    let mut stream = TcpStream::connect(&*server::host()).unwrap();
    let body = mock.response();

    stream.write_all(b"POST /mockito HTTP/1.1\n").unwrap_or(());
    stream.write_all(format!("x-mock-method: {}\n", mock.method).as_bytes()).unwrap_or(());
    stream.write_all(format!("x-mock-path: {}\n", mock.path).as_bytes()).unwrap_or(());
    for (field, value) in mock.headers.iter() {
        stream.write_all(format!("x-mock-{}: {}\n", field, value).as_bytes()).unwrap_or(());
    }
    stream.write_all(format!("content-length: {}\n", body.len()).as_bytes()).unwrap_or(());
    stream.write_all(b"\n").unwrap_or(());
    stream.write_all(body.as_bytes()).unwrap_or(());
}

pub fn reset() {
    let mut stream = TcpStream::connect(&*server::host()).unwrap();

    stream.write_all(b"DELETE /mockito HTTP/1.1\n\n").unwrap_or(());
}
