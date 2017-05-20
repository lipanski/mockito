extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Write, BufRead, BufReader};
use std::str::FromStr;
use mockito::{SERVER_ADDRESS, mock, reset, Matcher};

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

        if header_line.starts_with("Content-Length:") {
            let mut parts = header_line.split(":");
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

    let (status_line, _, body) = request("GET /hello", "");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", status_line);
    assert_eq!(mocked_body, body);
}

#[test]
fn test_two_route_mocks() {
    reset();

    mock("GET", "/a").with_body("aaa").create();
    mock("GET", "/b").with_body("bbb").create();

    let (_, _, body_a) = request("GET /a", "");

    assert_eq!("aaa", body_a);
    let (_, _, body_b) = request("GET /b", "");
    assert_eq!("bbb", body_b);
}

#[test]
fn test_no_match_returns_501() {
    reset();

    mock("GET", "/").with_body("matched").create();

    let (status_line, _, _) = request("GET /nope", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_header() {
    reset();

    mock("GET", "/")
        .match_header("content-type", "application/json")
        .with_body("{}")
        .create();

    mock("GET", "/")
        .match_header("content-type", "text/plain")
        .with_body("hello")
        .create();

    let (_, _, body_json) = request("GET /", "content-type: application/json\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_text) = request("GET /", "content-type: text/plain\r\n");
    assert_eq!("hello", body_text);
}

#[test]
fn test_match_header_is_case_insensitive_on_the_field_name() {
    reset();

    mock("GET", "/").match_header("content-type", "text/plain").create();

    let (uppercase_status_line, _, _) = request("GET /", "Content-Type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", uppercase_status_line);

    let (lowercase_status_line, _, _) = request("GET /", "content-type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", lowercase_status_line);
}

#[test]
fn test_match_multiple_headers() {
    reset();

    mock("GET", "/")
        .match_header("Content-Type", "text/plain")
        .match_header("Authorization", "secret")
        .with_body("matched")
        .create();

    let (_, _, body_matching) = request("GET /", "content-type: text/plain\r\nauthorization: secret\r\n");
    assert_eq!("matched", body_matching);

    let (status_not_matching, _, _) = request("GET /", "content-type: text/plain\r\nauthorization: meh\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_not_matching);
}

#[test]
fn test_match_header_any_matching() {
    reset();

    mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (_, _, body) = request("GET /", "content-type: something\r\n");
    assert_eq!("matched", body);
}

#[test]
fn test_match_header_any_not_matching() {
    reset();

    mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (status, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_header_missing_matching() {
    reset();

    mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", status);
}

#[test]
fn test_match_header_missing_not_matching() {
    reset();

    mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Authorization: something\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_matching() {
    reset();

    mock("GET", "/")
        .match_header("Hello", "World")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Hello: World\r\nContent-Type: something\r\n");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_not_matching() {
    reset();

    mock("GET", "/")
        .match_header("hello", "world")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Hello: World\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_mock_with_status() {
    reset();

    mock("GET", "/")
        .with_status(204)
        .with_body("")
        .create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 204 <unknown status code>\r\n", status_line);
}

#[test]
fn test_mock_with_header() {
    reset();

    mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let (_, headers, _) = request("GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
}

#[test]
fn test_mock_with_multiple_headers() {
    reset();

    mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_header("x-api-key", "1234")
        .with_body("{}")
        .create();

    let (_, headers, _) = request("GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
    assert!(headers.contains(&"x-api-key: 1234".to_string()));
}

#[test]
fn test_mock_preserves_header_order() {
    reset();

    let mut expected_headers = Vec::new();
    let mut mock = mock("GET", "/");

    // Add a large number of headers so getting the same order accidentally is unlikely.
    for i in 0..100 {
        let field = format!("x-custom-header-{}", i);
        let value = "test";
        mock.with_header(&field, value);
        expected_headers.push(format!("{}: {}", field, value));
    }

    mock.create();

    let (_, headers, _) = request("GET /", "");
    let custom_headers: Vec<_> = headers.into_iter()
        .filter(|header| header.starts_with("x-custom-header"))
        .collect();
    assert_eq!(custom_headers, expected_headers);
}

#[test]
fn test_reset_clears_mocks() {
    reset();

    mock("GET", "/reset").create();

    let (working_status_line, _, _) = request("GET /reset", "");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", working_status_line);

    reset();

    let (reset_status_line, _, _) = request("GET /reset", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}

#[test]
fn test_mock_remove_clears_the_mock() {
    reset();

    let mut mock = mock("GET", "/");
    mock.create();

    let (working_status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", working_status_line);

    mock.remove();

    let (reset_status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}

#[test]
fn test_mock_create_for_is_only_available_during_the_closure_lifetime() {
    reset();

    mock("GET", "/").create_for( || {
        let (working_status_line, _, _) = request("GET /", "");
        assert_eq!("HTTP/1.1 200 <unknown status code>\r\n", working_status_line);
    });

    let (reset_status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}
