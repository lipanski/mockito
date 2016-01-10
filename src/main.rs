extern crate mockito;

use mockito::{server, mock};

fn main() {
    server::start(Some(1234));

    println!("server running at: {}", server::host());

    mock("GET", "/hello").respond_with("HTTP/1.1 403 Forbidden\n\n");
    mock("GET", "/hello").header("authorization", "basic something").respond_with("HTTP/1.1 200 OK\n\nhello world!");
    mock("GET", "/bye").respond_with("HTTP/1.1 200 OK\n\nbye world!");
    mock("GET", "/file").respond_with_file("sample");

    loop {}
}
