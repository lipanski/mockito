extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Take, Write};
use mockito::{SERVER_ADDRESS, mock};

#[test]
fn test_create_starts_the_server() {
    mock("GET", "/").with_body("hello").create();

    let stream = TcpStream::connect(SERVER_ADDRESS);
    assert!(stream.is_ok());
}

#[test]
fn test_with_body_sets_the_correct_response() {
    let mut mock = mock("GET", "/");
    mock.with_body("hello");

    assert!(mock.response_body().is_some());
    assert_eq!("hello", mock.response_body().unwrap());
}

#[test]
fn test_with_body_from_file_sets_the_correct_response() {
    let mut mock = mock("GET", "/");
    mock.with_body_from_file("tests/files/simple.http");

    assert!(mock.response_body().is_some());
    assert_eq!("HTTP/1.1 200 OK\n\n", mock.response_body().unwrap());
}


#[test]
fn test_one_mock() {
    let mocked_response = "HTTP/1.1 200 OK\ncontent-length: 5\n\nhello";
    mock("GET", "/hello").with_header("hello", "world").with_body(mocked_response).create();

    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    stream.write_all(b"GET /hello HTTP/1.1\n\n");

    let mut actual_response = String::new();
    stream.take(mocked_response.len() as u64).read_to_string(&mut actual_response);

    assert_eq!(mocked_response, actual_response);
}
