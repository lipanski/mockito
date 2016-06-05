extern crate mockito;

use std::net::TcpStream;
use std::collections::HashMap;
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
    mock("GET", "/").respond_with_file("tests/files/simple.txt");

    let stream = TcpStream::connect(SERVER_ADDRESS);
    assert!(stream.is_ok());
}

#[test]
fn test_respond_with_file_sets_the_correct_response() {
    let mut mock = mock("GET", "/");
    mock.respond_with_file("tests/files/simple.txt");

    assert!(mock.response().is_some());
    assert_eq!("file contents\n", mock.response().unwrap());
}
