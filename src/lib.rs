#![warn(missing_docs)]
#![doc(html_logo_url = "http://lipanski.github.io/mockito/logo.png",
    html_root_url = "http://lipanski.github.io/mockito/docs/mockito/index.html")]

//!
//! Hello world
//!

extern crate hyper;
extern crate rustc_serialize;
extern crate rand;

mod server;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::Drop;
use hyper::client::Client;
use hyper::server::Request;
use hyper::header::{Headers, ContentType, Connection};
use rustc_serialize::json;
use rustc_serialize::{Encodable};
use rand::{thread_rng, Rng};

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
/// Initializes a mock for the provided `method` and `path`.
///
/// The mock is registered to the server only after the `create()` method has been called.
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
pub fn mock(method: &str, path: &str) -> Mock {
    Mock::new(method, path)
}

///
/// Removes all the mocks stored on the server.
///
pub fn reset() {
    server::try_start();

    Client::new()
        .delete(&[SERVER_URL, "/mocks"].join(""))
        .send()
        .unwrap();
}

///
/// Stores information about a mocked request. Should be initialized via `mockito::mock()`.
///
#[derive(PartialEq, Debug)]
pub struct Mock {
    id: String,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    response: MockResponse,
    _droppable: bool,
}

impl Mock {
    fn new(method: &str, path: &str) -> Self {
        Mock {
            id: thread_rng().gen_ascii_chars().take(24).collect(),
            method: method.to_owned().to_uppercase(),
            path: path.to_owned(),
            headers: HashMap::new(),
            response: MockResponse::new(),
            _droppable: true,
        }
    }

    ///
    /// Allows matching a particular request header when responding with a mock.
    ///
    /// When matching a request, the field letter case is ignored.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").match_header("content-type", "application/json");
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
    ///   .match_header("content-type", "application/json")
    ///   .match_header("authorization", "password");
    /// ```
    ///
    pub fn match_header(&mut self, field: &str, value: &str) -> &mut Self {
        self.headers.insert(field.to_owned(), value.to_owned());

        self
    }

    ///
    /// Sets the status code of the mock response. The default status code is 200.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_status(201);
    /// ```
    ///
    pub fn with_status(&mut self, status: usize) -> &mut Self {
        self.response.status = status;

        self
    }

    ///
    /// Sets a header of the mock response.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_header("content-type", "application/json");
    /// ```
    ///
    pub fn with_header(&mut self, field: &str, value: &str) -> &mut Self {
        self.response.headers.insert(field.to_owned(), value.to_owned());

        self
    }

    ///
    /// Sets the body of the mock response. Its `Content-Length` is handled automatically.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_body("hello world");
    /// ```
    ///
    pub fn with_body(&mut self, body: &str) -> &mut Self {
        self.response.body = body.to_owned();

        self
    }

    ///
    /// Sets the body of the mock response from the contents of a file stored under `path`.
    /// Its `Content-Length` is handled automatically.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_body_from_file("tests/files/simple.http");
    /// ```
    ///
    pub fn with_body_from_file(&mut self, path: &str) -> &mut Self {
        let mut file = File::open(path).unwrap();
        let mut body = String::new();

        file.read_to_string(&mut body).unwrap();

        self.response.body = body;

        self
    }

    ///
    /// Registers the mock to the server - your mock will be served only after calling this method.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_body("hello world").create();
    /// ```
    ///
    pub fn create(&self) {
        server::try_start();

        let mut headers = Headers::new();
        headers.set_raw("x-mock-id", vec!(self.id.as_bytes().to_vec()));
        headers.set_raw("x-mock-method", vec!(self.method.as_bytes().to_vec()));
        headers.set_raw("x-mock-path", vec!(self.path.as_bytes().to_vec()));

        for (field, value) in &self.headers {
            headers.set_raw("x-mock-".to_string() + field, vec!(value.as_bytes().to_vec()));
        }

        let body = json::encode(&self.response).unwrap();
        Client::new()
            .post(&[SERVER_URL, "/mocks"].join(""))
            .headers(headers)
            .header(ContentType::json())
            .header(Connection::close())
            .body(&body)
            .send()
            .unwrap();
    }

    pub fn remove(&self) {
        server::try_start();

        let mut headers = Headers::new();
        headers.set_raw("x-mock-id", vec!(self.id.as_bytes().to_vec()));

        Client::new()
            .delete(&[SERVER_URL, "/mocks"].join(""))
            .headers(headers)
            .header(Connection::close())
            .send()
            .unwrap();
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
}

impl Drop for Mock {
    fn drop(&mut self) {
        if self._droppable { println!("dropping"); self.remove(); }
    }
}

const DEFAULT_RESPONSE_STATUS: usize = 200;

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq)]
struct MockResponse {
    status: usize,
    headers: HashMap<String, String>,
    body: String,
}

impl MockResponse {
    pub fn new() -> Self {
        MockResponse {
            status: DEFAULT_RESPONSE_STATUS,
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}
