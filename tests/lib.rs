extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Write, BufRead, BufReader};
use mockito::{SERVER_ADDRESS, mock, reset};

fn request(route: &str, headers: &str) -> TcpStream {
    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    let message = [route, " HTTP/1.1\r\n", headers, "\r\n"].join("");
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
    reset();

    let mocked_body = "world";
    mock("GET", "/hello").with_body(mocked_body).create();

    let stream = request("GET /hello", "");
    let (status_line, _, body) = parse_stream(stream, mocked_body.len());

    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", status_line);
    assert_eq!(mocked_body, body);
}

#[test]
fn test_two_route_mocks() {
    reset();

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
fn test_no_match_returns_501() {
    reset();

    mock("GET", "/").with_body("matched").create();

    let stream_not_matching = request("GET /nope", "");
    let(status_line, _, _) = parse_stream(stream_not_matching, 0);

    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_header_with_two_mocks() {
    reset();

    mock("GET", "/")
        .match_header("Content-Type", "application/json")
        .with_body("{}")
        .create();

    mock("GET", "/")
        .match_header("Content-Type", "text/plain")
        .with_body("hello")
        .create();

    let stream_json = request("GET /", "content-type: application/json\r\n");
    let (_, _, body_json) = parse_stream(stream_json, 2);

    assert_eq!("{}", body_json);

    let stream_text = request("GET /", "content-type: text/plain\r\n");
    let (_, _, body_text) = parse_stream(stream_text, 5);

    assert_eq!("hello", body_text);
}

#[test]
fn test_match_multiple_headers() {
    reset();

    mock("GET", "/")
        .match_header("Content-Type", "text/plain")
        .match_header("Authorization", "secret")
        .with_body("matched")
        .create();

    let stream_matching = request("GET /", "content-type: text/plain\r\nauthorization: secret\r\n");
    let (_, _, body_matching) = parse_stream(stream_matching, 7);

    assert_eq!("matched", body_matching);

    let stream_not_matching = request("GET /", "content-type: text/plain\r\nauthorization: meh\r\n");
    let (status_not_matching, _, _) = parse_stream(stream_not_matching, 0);

    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_not_matching);
}

#[test]
fn test_mock_with_status() {
    reset();

    mock("GET", "/")
        .with_status(204)
        .with_body("")
        .create();

    let stream = request("GET /", "");
    let (status_line, _, _) = parse_stream(stream, 0);

    assert_eq!("HTTP/1.1 204 <unknown status code>\r\n", status_line);
}

#[test]
fn test_mock_with_header() {
    reset();

    mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let stream = request("GET /", "");
    let (_, headers, _) = parse_stream(stream, 0);

    assert!(headers.contains(&"content-type: application/json\r\n".to_string()));
}

#[test]
fn test_mock_with_multiple_headers() {
    reset();

    mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_header("x-api-key", "1234")
        .with_body("{}")
        .create();

    let stream = request("GET /", "");
    let (_, headers, _) = parse_stream(stream, 0);

    assert!(headers.contains(&"content-type: application/json\r\n".to_string()));
    assert!(headers.contains(&"x-api-key: 1234\r\n".to_string()));
}

#[test]
fn test_reset_clears_mocks() {
    reset();

    mock("GET", "/reset").create();

    let working_stream = request("GET /reset", "");
    let (working_status_line, _, _) = parse_stream(working_stream, 0);

    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", working_status_line);

    reset();

    let reset_stream = request("GET /reset", "");
    let (reset_status_line, _, _) = parse_stream(reset_stream, 0);

    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}
