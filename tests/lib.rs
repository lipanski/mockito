extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Write, BufRead, BufReader};
use mockito::{SERVER_ADDRESS, mock, reset};

fn request(route: &str, headers: &str) -> TcpStream {
    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    let message = [route, " HTTP/1.1\n", headers, "\n"].join("");
    stream.write_all(message.as_bytes()).unwrap();

    stream
}

fn parse_stream(stream: TcpStream, content_length: usize) -> (String, Vec<String>, String) {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    let mut headers = vec![];
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).unwrap();

        if header_line == "\r\n" { break }
        else { headers.push(header_line); }
    }

    let mut body = String::new();
    reader.take(content_length as u64).read_to_string(&mut body).unwrap();

    (status_line, headers, body)
}

#[test]
fn test_create_starts_the_server() {
    mock("GET", "/").with_body("hello").create();

    let stream = TcpStream::connect(SERVER_ADDRESS);
    assert!(stream.is_ok());
}

#[test]
fn test_simple_route_mock() {
    let mocked_body = "world";
    mock("GET", "/hello2").with_body(mocked_body).create();

    let stream = request("GET /hello2", "");
    let (status_line, _, body) = parse_stream(stream, mocked_body.len());

    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", status_line);
    assert_eq!(mocked_body, body);
}

#[test]
fn test_two_route_mocks() {
    mock("GET", "/a").with_body("aaa").create();
    mock("GET", "/b").with_body("bbb").create();

    let stream_a = request("GET /a", "");
    let (_, _, body_a) = parse_stream(stream_a, 3);

    assert_eq!("aaa", body_a);

    let stream_b = request("GET /b", "");
    let (_, _, body_b) = parse_stream(stream_b, 3);

    assert_eq!("bbb", body_b);
}

#[test]
fn test_header_matching_mock_fails_against_different_header_value() {
    reset();

    mock("GET", "/hello")
        .match_header("content-type", "application/json")
        .with_body("world")
        .create();

    let stream = request("GET /hello", "content-type: text/html\n");
    let (status, _, _) = parse_stream(stream, 0);

    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}
