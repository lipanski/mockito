#![feature(test)]

extern crate test;
extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Write, BufRead, BufReader};
use std::str::FromStr;
use test::Bencher;
use mockito::{SERVER_ADDRESS, mock, reset};

fn request_stream(route: &str, headers: &str) -> TcpStream {
    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    let message = [route, " HTTP/1.1\r\n", headers, "\r\n"].join("");
    stream.write_all(message.as_bytes()).unwrap();

    stream
}

fn parse_stream(stream: TcpStream) -> (String, Vec<String>, String) {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    let mut headers = vec![];
    let mut content_length: u64 = 0;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).unwrap();

        if header_line == "\r\n" { break }

        if header_line.starts_with("content-length:") {
            let mut parts = header_line.split(':');
            content_length = u64::from_str(parts.nth(1).unwrap().trim()).unwrap();
        }

        headers.push(header_line.trim_right().to_string());
    }

    let mut body = String::new();
    reader.take(content_length).read_to_string(&mut body).unwrap();

    (status_line, headers, body)
}

fn request(route: &str, headers: &str) -> (String, Vec<String>, String) {
    parse_stream(request_stream(route, headers))
}

#[bench]
fn bench_create_simple_mock(b: &mut Bencher) {
    reset();

    b.iter(|| {
        let _m = mock("GET", "/").with_body("test").create();
    })
}

#[bench]
fn bench_match_simple_mock(b: &mut Bencher) {
    reset();

    let _m = mock("GET", "/").with_body("test").create();

    b.iter(|| {
        let (status_line, _, _) = request("GET /", "");
        assert!(status_line.starts_with("HTTP/1.1 200"));
    })
}
