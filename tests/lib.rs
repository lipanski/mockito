extern crate mockito;

use std::net::TcpStream;
use std::io::{Read, Take, Write};
use mockito::{SERVER_ADDRESS, mock};

#[test]
fn test_respond_with_starts_the_server() {
    mock("GET", "/").respond_with("hello");

    let stream = TcpStream::connect(SERVER_ADDRESS);
    assert!(stream.is_ok());
}

#[test]
fn test_respond_with_sets_the_correct_response() {
    let mut mock = mock("GET", "/");
    mock.respond_with("hello");

    assert!(mock.response().is_some());
    assert_eq!("hello", mock.response().unwrap());
}

#[test]
fn test_respond_with_file_starts_the_server() {
    mock("GET", "/").respond_with_file("tests/files/simple.http");

    let stream = TcpStream::connect(SERVER_ADDRESS);
    assert!(stream.is_ok());
}

#[test]
fn test_respond_with_file_sets_the_correct_response() {
    let mut mock = mock("GET", "/");
    mock.respond_with_file("tests/files/simple.http");

    assert!(mock.response().is_some());
    assert_eq!("HTTP/1.1 200 OK\n\n", mock.response().unwrap());
}


#[test]
fn test_one_mock() {
    let mocked_response = "HTTP/1.1 200 OK\ncontent-length: 5\n\nhello";
    mock("GET", "/hello").respond_with(mocked_response);

    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    stream.write_all(b"GET /hello HTTP/1.1\n\n");

    let mut actual_response = String::new();
    stream.take(mocked_response.len() as u64).read_to_string(&mut actual_response);

    assert_eq!(mocked_response, actual_response);
}
