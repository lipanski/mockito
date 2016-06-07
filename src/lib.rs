#![warn(missing_docs)]
#![doc(html_logo_url = "http://lipanski.github.io/mockito/logo.png", html_root_url = "http://lipanski.github.io/mockito/docs/mockito/index.html")]

//!
//! Hello world
//!

extern crate hyper;

mod server;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use hyper::client::Client;
use hyper::server::Request;
use hyper::header::{Headers, ContentLength};

///
/// Points to the address the mock server is running at.
/// Can be used with `std::net::TcpStream`.
///
pub const SERVER_ADDRESS: &'static str = "0.0.0.0:1234";

///
/// Points to the URL the mock server is running at.
///
pub const SERVER_URL: &'static str = "http://0.0.0.0:1234";

///
/// Creates a mock for the provided `method` and `path`.
///
/// # Example
///
/// ```
/// use mockito::mock;
///
/// mock("GET", "/");
/// mock("POST", "/users");
/// mock("DELETE", "/users?id=1");
/// ```
///
/// The mock is sent to the server only after `respond_with()` or `respond_with_file()` are called
/// on the returned value.
///
pub fn mock(method: &str, path: &str) -> Mock {
    Mock::new(method, path)
}

///
/// Removes all the mocks stored on the server.
///
/// Because Rust tests run within the same process, the mock server won't be restarted with every
/// new test call. This method allows clearing mocks between tests. However, do note that `mockito`
/// will always try to match the last recorded mock so you might not need this method at all.
///
pub fn reset() {
    server::try_start();

    Client::new()
        .delete(&[SERVER_URL, "/mocks"].join(""))
        .send()
        .unwrap();
}

///
/// Stores information about a mocked request.
/// Should be initialized via `mockito::mock()`.
///
#[derive(PartialEq, Debug)]
pub struct Mock {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    response: Option<String>,
}

impl Mock {
    fn new(method: &str, path: &str) -> Self {
        Mock {
            method: method.to_owned().to_uppercase(),
            path: path.to_owned(),
            headers: HashMap::new(),
            response: None,
        }
    }

    ///
    /// Allows mocking a particular header based on `field` (the header field name) and `value`.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").header("content-type", "application/json");
    /// ```
    ///
    /// Like most other `Mock` methods, it allows chanining:
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/")
    ///   .header("content-type", "application/json")
    ///   .header("authorization", "password");
    /// ```
    ///
    pub fn header(&mut self, field: &str, value: &str) -> &mut Self {
        self.headers.insert(field.to_owned(), value.to_owned());

        self
    }

    ///
    /// Sets the response returned by the mock server to the value of `response`. This value should be valid HTTP.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").respond_with("HTTP/1.1 200 OK\n\n");
    /// ```
    ///
    pub fn respond_with(&mut self, response: &str) -> &mut Self {
        self.response = Some(response.to_owned());
        self.create();

        self
    }

    ///
    /// Sets the response returned by the mock server to the contents of the file stored under `path`.
    ///
    /// The contents of this file should be valid HTTP.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").respond_with_file("tests/files/simple.txt");
    /// ```
    ///
    pub fn respond_with_file(&mut self, path: &str) -> &mut Self {
        let mut file = File::open(path).unwrap();
        let mut response = String::new();

        file.read_to_string(&mut response).unwrap();

        self.response = Some(response);
        self.create();

        self
    }

    #[allow(missing_docs)]
    pub fn set_response(&mut self, response: String) {
        self.response = Some(response);
    }

    #[allow(missing_docs)]
    pub fn response(&self) -> Option<&String> {
        self.response.as_ref()
    }

    #[allow(missing_docs)]
    pub fn matches(&self, request: &Request) -> bool {
        self.method_matches(request)
            && self.path_matches(request)
            && self.headers_match(request)
    }

    fn method_matches(&self, request: &Request) -> bool {
        self.method == request.method.to_string().to_uppercase()
    }

    fn path_matches(&self, request: &Request) -> bool {
        self.path == request.uri.to_string()
    }

    fn headers_match(&self, request: &Request) -> bool {
        for (field, value) in self.headers.iter() {
            match request.headers.get_raw(&field) {
                Some(request_header_value) => {
                    let bytes: Vec<u8> = request_header_value.iter().flat_map(|el| el.iter().cloned()).collect();

                    if value == &String::from_utf8(bytes).unwrap() { continue }

                    return false
                },
                _ => return false
            }
        }

        true
    }

    fn create(&self) {
        server::try_start();

        let client = Client::new();

        let request = client.post(&[SERVER_URL, "/mocks"].join(""));
        let mut headers = Headers::new();

        headers.set_raw("x-mock-method", vec!(self.method.as_bytes().to_vec()));
        headers.set_raw("x-mock-path", vec!(self.path.as_bytes().to_vec()));

        for (field, value) in &self.headers {
            headers.set_raw("x-mock-".to_string() + field, vec!(value.as_bytes().to_vec()));
        }

        let body = self.response().unwrap();
        headers.set(ContentLength(body.len() as u64));

        request
            .headers(headers)
            .body(body)
            .send()
            .unwrap();
    }
}
