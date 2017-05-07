#![warn(missing_docs)]
#![doc(html_logo_url = "http://lipanski.github.io/mockito/logo/logo-black.png",
    html_root_url = "http://lipanski.github.io/mockito/generated/mockito/index.html")]

//!
//! Mockito is a library for creating HTTP mocks to be used in integration tests or for offline work.
//! It runs an HTTP server on your local port 1234 and can register and remove mocks.
//!
//! The server is run on a separate thread within the same process and will be cleaned up
//! at the end of the run.
//!
//! # Getting Started
//!
//! Using compiler flags, set the URL of your web client to `mockito::SERVER_URL` or `mockito::SERVER_ADDRESS`.
//!
//! ## Example
//!
//! ```
//! #[cfg(test)]
//! use mockito;
//!
//! #[cfg(not(test))]
//! const URL: &'static str = "https://api.twitter.com";
//!
//! #[cfg(test)]
//! const URL: &'static str = mockito::SERVER_URL;
//! ```
//!
//! Then start mocking:
//!
//! ## Example
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!   use mockito::mock;
//!
//!   #[test]
//!   fn test_something() {
//!     mock("GET", "/hello")
//!       .with_status(201)
//!       .with_header("content-type", "text/plain")
//!       .with_header("x-api-key", "1234")
//!       .with_body("world")
//!       .create();
//!
//!     // Any calls to GET /hello beyond this line will respond with 201, the
//!     // `content-type: text/plain` header and the body "world".
//!   }
//! }
//! ```
//!
//! # Run your tests
//!
//! Due to the nature of this library (all your mocks are recorded on the same server running in background),
//! it is highly recommended that you **run your tests on a single thread**:
//!
//! ```sh
//! cargo test -- --test-threads=1
//!
//! # Same, but using an environment variable
//! RUST_TEST_THREADS=1 cargo test
//! ```
//!
//! In some situations, when you're *always* testing/mocking different routes and never need to reset
//! or override the existing mocks, you might get away with running your tests on multiple threads.
//!
//! # Matching by header
//!
//! Mockito currently matches by method and path, but also by headers. The header field letter case is ignored.
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! mock("GET", "/hello")
//!   .match_header("content-type", "application/json")
//!   .with_body("{'hello': 'world'}")
//!   .create();
//!
//! mock("GET", "/hello")
//!   .match_header("content-type", "text/plain")
//!   .with_body("world")
//!   .create();
//!
//! // JSON requests to GET /hello will respond with JSON, while plain requests
//! // will respond with text.
//! ```
//!
//! # Other header matchers
//!
//! You can match a header *only by its field name*, by setting the `Mock::match_header` value to `Matcher::Any`.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! mock("GET", "/hello")
//!  .match_header("content-type", Matcher::Any)
//!  .with_body("something");
//!
//! // Requests containing any content-type header value will be mocked.
//! // Requests not containing this header will return `501 Not Implemented`.
//! ```
//!
//! You can mock requests that should be *missing a particular header field*, by setting the `Mock::match_header`
//! value to `Matcher::Missing`.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! mock("GET", "/hello")
//!   .match_header("authorization", Matcher::Missing)
//!   .with_body("no authorization header");
//!
//! // Requests without the authorization header will be matched.
//! // Requests containing the authorization header will return `501 Not Implemented`.
//! ```
//!
//! # Non-matching calls
//!
//! Any calls to the Mockito server that are not matched will return *501 Not Implemented*.
//!
//! # Cleaning up
//!
//! Even though **mocks are matched in reverse order** (most recent one wins), in some situations
//! it might be useful to clean up right after the test. There are multiple ways of doing this.
//!
//! By using a closure:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! mock("GET", "/hello")
//!   .with_body("world")
//!   .create_for(|| {
//!     // mock only valid for the lifetime of this closure
//!     // NOTE: it might still be accessible by separate threads
//!   });
//! ```
//!
//! By calling `remove()` on the mock:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! let mut mock = mock("GET", "/hello");
//! mock.with_body("world").create();
//!
//! // do your thing
//!
//! mock.remove();
//! ```
//!
//! By calling `reset()` to **remove all mocks**:
//!
//! ## Example
//!
//! ```
//! use mockito::reset;
//!
//! reset();
//! ```
//!

extern crate hyper;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate rand;

mod server;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::cmp::PartialEq;
use std::convert::{From, Into};
use hyper::client::Client;
use hyper::server::Request;
use hyper::header::{Headers, ContentType, ContentLength, Connection};
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

#[allow(missing_docs)]
pub fn start() {
    server::try_start();
}

///
/// Allows matching headers in multiple ways: matching the exact field name and value, matching only by field name
/// or matching that the field name is not present at all.
///
/// These matchers are used within the `Mock::match_header` call.
///
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Matcher {
    /// Given the header field, matches the exact header value. There's also an implementation of `From<&str>`
    /// to keep things simple and backwards compatible.
    Exact(String),
    /// Given the header field, matches any header value.
    Any,
    /// Matches when the header field is *not* be present in the request.
    Missing,
}

impl<'a> From<&'a str> for Matcher {
    fn from(value: &str) -> Matcher {
        Matcher::Exact(value.to_string())
    }
}

impl PartialEq<String> for Matcher {
    fn eq(&self, other: &String) -> bool {
        match self {
            &Matcher::Exact(ref value) => { value == other },
            &Matcher::Any => true,
            &Matcher::Missing => false,
        }
    }
}

///
/// Stores information about a mocked request. Should be initialized via `mockito::mock()`.
///
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Mock {
    id: String,
    method: String,
    path: String,
    headers: HashMap<String, Matcher>,
    body: Matcher,
    response: MockResponse,
}

impl Mock {
    fn new(method: &str, path: &str) -> Self {
        Mock {
            id: thread_rng().gen_ascii_chars().take(24).collect(),
            method: method.to_owned().to_uppercase(),
            path: path.to_owned(),
            headers: HashMap::new(),
            body: Matcher::Any,
            response: MockResponse::new(),
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
    pub fn match_header<M: Into<Matcher>>(&mut self, field: &str, value: M) -> &mut Self {
        self.headers.insert(field.to_owned(), value.into());

        self
    }

    pub fn match_body<M: Into<Matcher>>(&mut self, value: M) -> &mut Self {
        self.body = value.into();

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
    pub fn create(&mut self) -> &mut Self {
        server::try_start();

        // let mut headers = Headers::new();
        // headers.set_raw("x-mock-id", vec!(self.id.as_bytes().to_vec()));
        // headers.set_raw("x-mock-method", vec!(self.method.as_bytes().to_vec()));
        // headers.set_raw("x-mock-path", vec!(self.path.as_bytes().to_vec()));

        // for (field, value) in &self.headers {
        //     let (header_field, header_value) =
        //         match value {
        //             &Matcher::Missing => ("x-mock-header-missing".to_string(), field.as_bytes()),
        //             &Matcher::Any => ("x-mock-header-any".to_string(), field.as_bytes()),
        //             &Matcher::Exact(ref exact_value) => ("x-mock-".to_string() + field, exact_value.as_bytes()),
        //         };

        //     headers.set_raw(header_field, vec!(header_value.to_vec()));
        // }

        let body = serde_json::to_string(&self).unwrap();
        println!("{:?}", body);
        Client::new()
            .post(&[SERVER_URL, "/mocks"].join(""))
            .header(ContentType::json())
            .header(Connection::close())
            .body(&body)
            .send()
            .unwrap();

        self
    }

    ///
    /// Registers the mock to the server, executes the passed closure and removes the mock afterwards.
    ///
    /// **NOTE:** During the closure lifetime, the mock might still be available to seperate threads.
    ///
    /// # Example
    ///
    /// ```
    /// use std::thread::{sleep};
    /// use std::time::Duration;
    /// use mockito::mock;
    ///
    /// mock("GET", "/").with_body("hello world").create_for(|| {
    ///   // This mock will only be available for the next 1 second
    ///   sleep(Duration::new(1, 0));
    /// });
    /// ```
    ///
    pub fn create_for<F: Fn() -> ()>(&mut self, environment: F) -> &mut Self {
        self.create();
        environment();
        self.remove();

        self
    }

    ///
    /// Removes the current mock from the server.
    ///
    /// # Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let mut mock = mock("GET", "/");
    /// mock.with_body("hello world").create();
    ///
    /// // stuff
    ///
    /// mock.remove();
    /// ```
    ///
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
    pub fn matches(&self, request: &mut Request) -> bool {
        self.method_matches(request)
            && self.path_matches(request)
            && self.headers_match(request)
            && self.body_matches(request)
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
                None => {
                    if value == &Matcher::Missing { continue }

                    return false
                },
            }
        }

        true
    }

    fn body_matches(&self, request: &mut Request) -> bool {
        if self.body == Matcher::Any { return true };

        let content_length: ContentLength = match request.headers.get() {
            Some(value) => *value,
            None => { return self.body == Matcher::Missing },
        };

        let mut body = String::new();
        request.take(content_length.0).read_to_string(&mut body).unwrap();

        self.body == body
    }
}

const DEFAULT_RESPONSE_STATUS: usize = 200;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
