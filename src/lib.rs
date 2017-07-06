#![warn(missing_docs)]
#![doc(html_logo_url = "http://lipanski.github.io/mockito/logo/logo-black.png",
    html_root_url = "http://lipanski.github.io/mockito/generated/mockito/index.html")]

//!
//! Mockito is a library for creating HTTP mocks to be used in integration tests or for offline work.
//! It runs an HTTP server on your local port 1234 which delivers, creates and remove the mocks.
//!
//! The server is run on a separate thread within the same process and will be removed
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
//!     let _m = mock("GET", "/hello")
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
//! # Lifetime
//!
//! Just like any Rust object, a mock is available only through its lifetime. You'll want to assign
//! the mocks to variables in order to extend and control their lifetime.
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! let _m1 = mock("GET", "/long").with_body("hello").create();
//!
//! {
//!     let _m2 = mock("GET", "/short").with_body("hi").create();
//!
//!     // Requests to GET /short will be mocked til here
//! }
//!
//! // Requests to GET /long will be mocked til here
//! ```
//!
//! Note how I **didn't use the same variable name** for both mocks (e.g. `let _`), as it would have ended the
//! lifetime of the first mock with the second assignment.
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
//! # Asserts
//!
//! You can use the `Mock::assert` method to **assert that a mock was called**. By default, the method expects
//! that only one request to your mock was triggered.
//!
//! ## Example
//!
//! ```
//! extern crate mockito;
//! extern crate curl;
//!
//! let mock = mockito::mock("GET", "/hello").create();
//!
//! let mut request = curl::easy::Easy::new();
//! request.url(&[mockito::SERVER_URL, "/hello"].join("")).unwrap();
//! request.perform().unwrap();
//!
//! mock.assert();
//! ```
//!
//! If you're expecting more than 1 request, you can use the `Mock::expect` method to specify the exact amout of requests:
//!
//! ## Example
//!
//! ```
//! extern crate mockito;
//! extern crate curl;
//!
//! let mock = mockito::mock("GET", "/hello").expect(3).create();
//!
//! for _ in 0..3 {
//!     let mut request = curl::easy::Easy::new();
//!     request.url(&[mockito::SERVER_URL, "/hello"].join("")).unwrap();
//!     request.perform().unwrap();
//! }
//!
//! mock.assert();
//! ```
//!
//! # Matchers
//!
//! Mockito can match your request by method, path, headers or body.
//!
//! Various matchers are provided by the `Matcher` type: exact, partial (regular expressions), any or missing.
//!
//! # Matching by path
//!
//! By default, the request path is compared by its exact value:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! // Matched only calls to GET /hello
//! let _m = mock("GET", "/hello").create();
//! ```
//!
//! You can also match the path partially, by using a regular expression:
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match calls to GET /hello/1 and GET /hello/2
//! let _m = mock("GET", Matcher::Regex(r"^/hello/(1|2)$".to_string())).create();
//! ```
//!
//! Or you can catch all requests, by using the `Matcher::Any` variant:
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match any GET request
//! let _m = mock("GET", Matcher::Any).create();
//! ```
//!
//! # Matching by header
//!
//! By default, headers are compared by their exact value. The header name letter case is ignored though.
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! let _m1 = mock("GET", "/hello")
//!   .match_header("content-type", "application/json")
//!   .with_body("{'hello': 'world'}")
//!   .create();
//!
//! let _m2 = mock("GET", "/hello")
//!   .match_header("content-type", "text/plain")
//!   .with_body("world")
//!   .create();
//!
//! // JSON requests to GET /hello will respond with JSON, while plain requests
//! // will respond with text.
//! ```
//!
//! You can also match a header value with a *regular expressions*, by using the `Matcher::Regex` matcher:
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! let _m = mock("GET", "/hello")
//!   .match_header("content-type", Matcher::Regex(r".*json.*".to_string()))
//!   .with_body("{'hello': 'world'}")
//!   .create();
//! ```
//!
//! Or you can match a header *only by its field name*, by setting the `Mock::match_header` value to `Matcher::Any`.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! let _m = mock("GET", "/hello")
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
//! let _m = mock("GET", "/hello")
//!   .match_header("authorization", Matcher::Missing)
//!   .with_body("no authorization header");
//!
//! // Requests without the authorization header will be matched.
//! // Requests containing the authorization header will return `501 Not Implemented`.
//! ```
//!
//! # Matching by body
//!
//! You can match a request by its body by using the `Mock#match_body` method.
//! By default the request body is ignored, similar to passing the `Matcher::Any` argument to the `match_body` method.
//!
//! You can match a body by an exact value:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! // Will match requests to POST / whenever the request body is "hello"
//! let _m = mock("POST", "/").match_body("hello").create();
//! ```
//!
//! Or you can match the body by using a regular expression:
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match requests to POST / whenever the request body *contains* the word "hello" (e.g. "hello world")
//! let _m = mock("POST", "/").match_body(Matcher::Regex("hello".to_string())).create();
//! ```
//!
//! # Non-matching calls
//!
//! Any calls to the Mockito server that are not matched will return *501 Not Implemented*.
//!
//! Note that **mocks are matched in reverse order** - the most recent one wins.
//!
//! # Cleaning up
//!
//! As mentioned earlier, mocks are cleaned up at the end of their normal Rust lifetime. However,
//! you can always use the `reset` method to clean up *all* the mocks registered so far.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, reset};
//!
//! let _m1 = mock("GET", "/1").create();
//! let _m2 = mock("GET", "/2").create();
//! let _m3 = mock("GET", "/3").create();
//!
//! reset();
//!
//! // Nothing is mocked at this point
//! ```
//!
//! Or you can use `std::mem::drop` to remove a single mock without having to wait for its scope to end:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//! use std::mem;
//!
//! let m = mock("GET", "/hello").create();
//!
//! // Requests to GET /hello are mocked
//!
//! mem::drop(m);
//!
//! // Still in the scope of `m`, but requests to GET /hello aren't mocked any more
//!

extern crate curl;
extern crate http_muncher;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate rand;
extern crate regex;
#[macro_use] extern crate lazy_static;

mod server;
mod request;
type Request = request::Request;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::cmp::PartialEq;
use std::convert::{From, Into};
use std::ops::Drop;
use std::fmt;
use curl::easy::Easy;
use rand::{thread_rng, Rng};
use regex::Regex;

///
/// Points to the address the mock server is running at.
/// Can be used with `std::net::TcpStream`.
///
pub const SERVER_ADDRESS: &'static str = "127.0.0.1:1234";

///
/// Points to the URL the mock server is running at.
///
pub const SERVER_URL: &'static str = "http://127.0.0.1:1234";

///
/// Initializes a mock for the provided `method` and `path`.
///
/// The mock is registered to the server only after the `create()` method has been called.
///
/// ## Example
///
/// ```
/// use mockito::mock;
///
/// let _m1 = mock("GET", "/");
/// let _m2 = mock("POST", "/users");
/// let _m3 = mock("DELETE", "/users?id=1");
/// ```
///
pub fn mock<P: Into<Matcher>>(method: &str, path: P) -> Mock {
    Mock::new(method, path)
}

///
/// Removes all the mocks stored on the server.
///
pub fn reset() {
    server::try_start();

    let mut request = Easy::new();
    request.url(&[SERVER_URL, "/mocks"].join("")).unwrap();
    request.custom_request("DELETE").unwrap();
    request.perform().unwrap();
}

#[allow(missing_docs)]
pub fn start() {
    server::try_start();
}

///
/// Allows matching the request path or headers in multiple ways: matching the exact value, matching any value (as
/// long as it is present), matching by regular expression or checking that a particular header is missing.
///
/// These matchers are used within the `mock` and `Mock::match_header` calls.
///
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Matcher {
    /// Matches the exact path or header value. There's also an implementation of `From<&str>`
    /// to keep things simple and backwards compatible.
    Exact(String),
    /// Matches a path or header value by a regular expression.
    Regex(String),
    /// Matches any path or any header value.
    Any,
    /// Checks that a header is not present in the request.
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
            &Matcher::Regex(ref regex) => { Regex::new(regex).unwrap().is_match(other) },
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
    path: Matcher,
    headers: HashMap<String, Matcher>,
    body: Matcher,
    response: MockResponse,
    hits: usize,
    expected_hits: usize,
}

impl Mock {
    fn new<P: Into<Matcher>>(method: &str, path: P) -> Self {
        Mock {
            id: thread_rng().gen_ascii_chars().take(24).collect(),
            method: method.to_owned().to_uppercase(),
            path: path.into(),
            headers: HashMap::new(),
            body: Matcher::Any,
            response: MockResponse::new(),
            hits: 0,
            expected_hits: 1,
        }
    }

    ///
    /// Allows matching a particular request header when responding with a mock.
    ///
    /// When matching a request, the field letter case is ignored.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").match_header("content-type", "application/json");
    /// ```
    ///
    /// Like most other `Mock` methods, it allows chanining:
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/")
    ///   .match_header("content-type", "application/json")
    ///   .match_header("authorization", "password");
    /// ```
    ///
    pub fn match_header<M: Into<Matcher>>(mut self, field: &str, value: M) -> Self {
        self.headers.insert(field.to_owned().to_lowercase(), value.into());

        self
    }

    ///
    /// Allows matching a particular request body when responding with a mock.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m1 = mock("POST", "/").match_body("{'hello':'world'}").with_body("json").create();
    /// let _m2 = mock("POST", "/").match_body("hello=world").with_body("form").create();
    ///
    /// // Requests passing "{'hello':'world'}" inside the body will be responded with "json".
    /// // Requests passing "hello=world" inside the body will be responded with "form".
    /// ```
    ///
    pub fn match_body<M: Into<Matcher>>(mut self, body: M) -> Self {
        self.body = body.into();

        self
    }

    ///
    /// Sets the status code of the mock response. The default status code is 200.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_status(201);
    /// ```
    ///
    pub fn with_status(mut self, status: usize) -> Self {
        self.response.status = status;

        self
    }

    ///
    /// Sets a header of the mock response.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_header("content-type", "application/json");
    /// ```
    ///
    pub fn with_header(mut self, field: &str, value: &str) -> Self {
        self.response.headers.push((field.to_owned(), value.to_owned()));

        self
    }

    ///
    /// Sets the body of the mock response. Its `Content-Length` is handled automatically.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_body("hello world");
    /// ```
    ///
    pub fn with_body(mut self, body: &str) -> Self {
        self.response.body = body.to_owned();

        self
    }

    ///
    /// Sets the body of the mock response from the contents of a file stored under `path`.
    /// Its `Content-Length` is handled automatically.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_body_from_file("tests/files/simple.http");
    /// ```
    ///
    pub fn with_body_from_file(mut self, path: &str) -> Self {
        let mut file = File::open(path).unwrap();
        let mut body = String::new();

        file.read_to_string(&mut body).unwrap();

        self.response.body = body;

        self
    }

    ///
    /// Sets the expected amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    /// Defaults to 1 request.
    ///
    pub fn expect(mut self, hits: usize) -> Self {
        self.expected_hits = hits;

        self
    }

    ///
    /// Asserts that the expected amount of requests (defaults to 1 request) were performed.
    ///
    pub fn assert(&self) {
        let remote = self.remote().expect("The request to retrieve the remote mock failed.");
        assert_eq!(self.expected_hits, remote.hits, "Expected {} request(s) to {}, but received {}", self.expected_hits, self, remote.hits);
    }

    ///
    /// Registers the mock to the server - your mock will be served only after calling this method.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_body("hello world").create();
    /// ```
    ///
    pub fn create(self) -> Self {
        server::try_start();

        let body = serde_json::to_string(&self).unwrap();

        let mut request = Easy::new();
        request.url(&[SERVER_URL, "/mocks"].join("")).unwrap();
        request.post(true).unwrap();
        request.post_fields_copy(body.as_bytes()).unwrap();
        request.perform().unwrap();

        self
    }

    ///
    /// Removes the current mock from the server.
    ///
    fn remove(&self) {
        server::try_start();

        let mut request = Easy::new();
        request.url(&[SERVER_URL, "/mocks/", &self.id].join("")).unwrap();
        request.custom_request("DELETE").unwrap();
        request.perform().unwrap();
    }

    ///
    /// Retrieves the remote copy of the current mock.
    /// Mainly used to sync the hit count.
    ///
    fn remote(&self) -> Result<Self, ()> {
        server::try_start();

        let mut buffer = Vec::new();

        let mut request = Easy::new();
        request.url(&[SERVER_URL, "/mocks/", &self.id].join("")).unwrap();

        {
            let mut transfer = request.transfer();

            transfer.write_function(|data| {
                buffer.extend_from_slice(data);
                Ok(data.len())
            }).unwrap();

            transfer.perform().unwrap();
        }

        serde_json::from_slice(&buffer).map_err(|_| ())
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.remove();
    }
}

impl fmt::Display for Mock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.path {
            Matcher::Exact(ref value) => write!(f, "{} {}", self.method, value),
            Matcher::Regex(ref value) => write!(f, "{} {}", self.method, value),
            Matcher::Any => write!(f, "{} *", self.method),
            Matcher::Missing => write!(f, "{} -", self.method),
        }
    }
}

const DEFAULT_RESPONSE_STATUS: usize = 200;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct MockResponse {
    status: usize,
    headers: Vec<(String, String)>,
    body: String,
}

impl MockResponse {
    pub fn new() -> Self {
        MockResponse {
            status: DEFAULT_RESPONSE_STATUS,
            headers: Vec::new(),
            body: String::new(),
        }
    }
}
