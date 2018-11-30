#![warn(missing_docs)]
#![doc(html_logo_url = "https://raw.githubusercontent.com/lipanski/mockito/master/docs/logo-black.png")]

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
//! const URL: &str = "https://api.twitter.com";
//!
//! #[cfg(test)]
//! const URL: &str = mockito::SERVER_URL;
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
//! Avoid using the underscore matcher when creating your mocks, as in `let _ = mock("GET", "/")`.
//! This will end your mock's lifetime immediately. You can still use the underscore to prefix your variable
//! names in an assignment, but don't limit it to just this one character.
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
//! Note how I **didn't use the same variable name** for both mocks (e.g. `let _m`), as it would have ended the
//! lifetime of the first mock with the second assignment.
//!
//! # Limitations
//!
//! Creating mocks from threads is currently not possible. Please use the main (test) thread for that.
//! See the note on threads at the end for more details.
//!
//! # Asserts
//!
//! You can use the `Mock::assert` method to **assert that a mock was called**. In other words, the
//! `Mock#assert` method can validate that your code performed the expected HTTP requests.
//!
//! By default, the method expects that only one request to your mock was triggered.
//!
//! ## Example
//!
//! ```no_run
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, SERVER_ADDRESS};
//!
//! let mock = mock("GET", "/hello").create();
//!
//! {
//!     // Place a request
//!     let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! mock.assert();
//! ```
//!
//! If you're expecting more than 1 request, you can use the `Mock::expect` method to specify the exact amount of requests:
//!
//! ## Example
//!
//! ```no_run
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, SERVER_ADDRESS};
//!
//! let mock = mockito::mock("GET", "/hello").expect(3).create();
//!
//! for _ in 0..3 {
//!     // Place a request
//!     let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! mock.assert();
//! ```
//!
//! The errors produced by the `assert` method contain information about the tested mock, but also about the
//! **last unmatched request**, which can be very useful to track down an error in your implementation or
//! a missing or incomplete mock. A coloured diff is also displayed.
//!
//! Here's an example of how a `Mock#assert` error looks like:
//!
//! ```text
//! > Expected 1 request(s) to:
//!
//! POST /users
//! bob
//!
//! ...but received 0
//!
//! > The last unmatched request was:
//!
//! POST /users
//! content-length: 5
//! alice
//!
//! > Difference:
//!
//! # A coloured diff
//!
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
//!  .with_body("something")
//!  .create();
//!
//! // Requests containing any content-type header value will be mocked.
//! // Requests not containing this header will return `501 Mock Not Found`.
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
//!   .with_body("no authorization header")
//!   .create();
//!
//! // Requests without the authorization header will be matched.
//! // Requests containing the authorization header will return `501 Mock Not Found`.
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
//! Or you can match the body using a JSON object:
//!
//! ## Example
//!
//! ```
//! # extern crate mockito;
//! #[macro_use]
//! extern crate serde_json;
//! use mockito::{mock, Matcher};
//!
//! # fn main() {
//! // Will match requests to POST / whenever the request body matches the json object
//! let _m = mock("POST", "/").match_body(Matcher::Json(json!({"hello":"world"}))).create();
//! # }
//! ```
//!
//! If `serde_json::json!` is not exposed, you can use `Matcher::JsonString` the same way,
//! but by passing a `String` to the matcher:
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match requests to POST / whenever the request body matches the json object
//! let _m = mock("POST", "/")
//!     .match_body(
//!        Matcher::JsonString("{\"hello\":\"world\"}".to_string())
//!     )
//!     .create();
//! ```
//!
//! # The `AnyOf` matcher
//!
//! The `Matcher::AnyOf` construct takes a vector of matchers as arguments and will be enabled
//! if at least one of the provided matchers matches the request.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match requests to POST / whenever the request body is either `hello=world` or `{"hello":"world"}`
//! let _m = mock("POST", "/")
//!     .match_body(
//!         Matcher::AnyOf(vec![
//!             Matcher::Exact("hello=world".to_string()),
//!             Matcher::JsonString("{\"hello\":\"world\"}".to_string()),
//!         ])
//!      )
//!     .create();
//!```
//!
//! # Non-matching calls
//!
//! Any calls to the Mockito server that are not matched will return *501 Mock Not Found*.
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
//! ```
//!
//! # Threads
//!
//! Mockito records all your mocks on the same server running in the background. For this
//! reason, Mockito tests are run sequentially. This is handled internally via a thread-local
//! mutex lock acquired **whenever you create a mock**. Tests that don't create mocks will
//! still be run in parallel.
//!

extern crate httparse;
extern crate rand;
extern crate regex;
#[macro_use] extern crate lazy_static;
extern crate serde_json;
extern crate difference;
extern crate colored;

#[macro_use] mod logger;
mod server;
mod request;
mod response;
mod diff;

type Request = request::Request;
type Response = response::Response;

use std::fs::File;
use std::io::Read;
use std::convert::{From, Into};
use std::ops::Drop;
use std::fmt;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use std::sync::{Mutex, LockResult, MutexGuard};
use std::cell::RefCell;

lazy_static! {
    // A global lock that ensure all Mockito tests are run on a single thread.
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

thread_local!(
    // A thread-local reference to the global lock. This is acquired within `Mock#create()`.
    static LOCAL_TEST_MUTEX: RefCell<LockResult<MutexGuard<'static, ()>>> = RefCell::new(TEST_MUTEX.lock());
);

///
/// Points to the address the mock server is running at.
/// Can be used with `std::net::TcpStream`.
///
pub const SERVER_ADDRESS: &str = "127.0.0.1:1234";

///
/// Points to the URL the mock server is running at.
///
pub const SERVER_URL: &str = "http://127.0.0.1:1234";

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

    let state_mutex = server::STATE.clone();
    let mut state = state_mutex.lock().unwrap();
    state.mocks.clear();
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
#[derive(Clone, PartialEq, Debug)]
#[allow(deprecated)] // Rust bug #38832
pub enum Matcher {
    /// Matches the exact path or header value. There's also an implementation of `From<&str>`
    /// to keep things simple and backwards compatible.
    Exact(String),
    /// Matches a path or header value by a regular expression.
    Regex(String),
    /// Matches a specified JSON body from a `serde_json::Value`
    Json(serde_json::Value),
    /// Matches a specified JSON body from a `String`
    JsonString(String),
    /// At least one must match
    AnyOf(Vec<Matcher>),
    /// Matches any path or any header value.
    Any,
    /// Checks that a header is not present in the request.
    Missing,
}

impl<'a> From<&'a str> for Matcher {
    fn from(value: &str) -> Self {
        Matcher::Exact(value.to_string())
    }
}

impl Matcher {
    fn matches_values(&self, header_values: &[&str]) -> bool {
        match self {
            Matcher::Missing => header_values.is_empty(),
            // AnyOf([…Missing…]) is handled here, but
            // AnyOf([Something]) is handled in the last block.
            // That's because Missing matches against all values at once,
            // but other matchers match against individual values.
            Matcher::AnyOf(ref matchers) if header_values.is_empty() => {
                matchers.iter().any(|m| m.matches_values(header_values))
            },
            _ => !header_values.is_empty() && header_values.iter().all(|val| self.matches_value(val)),
        }
    }

    #[allow(deprecated)]
    fn matches_value(&self, other: &str) -> bool {
        match self {
            Matcher::Exact(ref value) => { value == other },
            Matcher::Regex(ref regex) => { Regex::new(regex).unwrap().is_match(other) },
            Matcher::Json(ref json_obj) => {
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                *json_obj == other
            },
            Matcher::JsonString(ref value) => {
                let value: serde_json::Value = serde_json::from_str(value).unwrap();
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                value == other
            },
            Matcher::Any => true,
            Matcher::AnyOf(ref matchers) => {
                matchers.iter().any(|m| m.matches_value(other))
            },
            Matcher::Missing => false,
        }
    }
}

///
/// Stores information about a mocked request. Should be initialized via `mockito::mock()`.
///
#[derive(Clone, PartialEq, Debug)]
pub struct Mock {
    id: String,
    method: String,
    path: Matcher,
    headers: Vec<(String, Matcher)>,
    body: Matcher,
    response: Response,
    hits: usize,
    expected_hits: usize,
    is_remote: bool,
}

impl Mock {
    fn new<P: Into<Matcher>>(method: &str, path: P) -> Self {
        Self {
            id: thread_rng().sample_iter(&Alphanumeric).take(24).collect(),
            method: method.to_owned().to_uppercase(),
            path: path.into(),
            headers: Vec::new(),
            body: Matcher::Any,
            response: Response::default(),
            hits: 0,
            expected_hits: 1,
            is_remote: false,
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
        self.headers.push((field.to_owned().to_lowercase(), value.into()));

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
        self.response.status = status.into();

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
    pub fn with_body<StrOrBytes: AsRef<[u8]>>(mut self, body: StrOrBytes) -> Self {
        self.response.body = body.as_ref().to_owned();

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
        let mut body = Vec::new();

        file.read_to_end(&mut body).unwrap();

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
        let mut opt_hits = None;
        let mut opt_message = None;

        {
            let state_mutex = server::STATE.clone();
            let state = state_mutex.lock().unwrap();

            if let Some(remote_mock) = state.mocks.iter().find(|mock| mock.id == self.id) {
                opt_hits = Some(remote_mock.hits);

                let mut message = format!("\n> Expected {} request(s) to:\n{}\n...but received {}\n\n", self.expected_hits, self, remote_mock.hits);

                if let Some(last_request) = state.unmatched_requests.last() {
                    message.push_str(&format!("> The last unmatched request was:\n{}\n", last_request));

                    let difference = diff::compare(&self.to_string(), &last_request.to_string());
                    message.push_str(&format!("> Difference:\n{}\n", difference));
                }

                opt_message = Some(message);
            }
        }

        match (opt_hits, opt_message) {
            (Some(hits), Some(message)) => assert_eq!(self.expected_hits, hits, "{}", message),
            _ => panic!("Could not retrieve enough information about the remote mock."),
        }
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

        // Ensures Mockito tests are run sequentially.
        LOCAL_TEST_MUTEX.with(|_| {});

        let state_mutex = server::STATE.clone();
        let mut state = state_mutex.lock().unwrap();

        let mut remote_mock = self.clone();
        remote_mock.is_remote = true;
        state.mocks.push(remote_mock);

        debug!("Mock::create() called for {}", self);

        self
    }

    fn is_local(&self) -> bool {
        !self.is_remote
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        if self.is_local() {
            let state_mutex = server::STATE.clone();
            let mut state = state_mutex.lock().unwrap();

            if let Some(pos) = state.mocks.iter().position(|mock| mock.id == self.id) {
                state.mocks.remove(pos);
            }

            debug!("Mock::drop() called for {}", self);
        }
    }
}

impl fmt::Display for Mock {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatted = String::new();

        formatted.push_str("\r\n");
        formatted.push_str(&self.method);
        formatted.push_str(" ");

        match self.path {
            Matcher::Exact(ref value) => {
                formatted.push_str(value);
                formatted.push_str("\r\n");
            },
            Matcher::Regex(ref value) => {
                formatted.push_str(value);
                formatted.push_str(" (regex)\r\n")
            },
            Matcher::Json(ref json_obj) => {
                formatted.push_str(&json_obj.to_string());
                formatted.push_str(" (json)\r\n")
            },
            Matcher::JsonString(ref value) => {
                formatted.push_str(value);
                formatted.push_str(" (json)\r\n")
            },
            Matcher::Any => formatted.push_str("(any)\r\n"),
            Matcher::AnyOf(..) => formatted.push_str("(any of)\r\n"),
            Matcher::Missing => formatted.push_str("(missing)\r\n"),
        }

        for &(ref key, ref value) in &self.headers {
            match value {
                Matcher::Exact(ref value) => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str(value);
                },
                Matcher::Regex(ref value) => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str(value);
                    formatted.push_str(" (regex)")
                },
                Matcher::Json(ref json_obj) => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str(&json_obj.to_string());
                    formatted.push_str(" (json)")
                },
                Matcher::JsonString(ref value) => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str(value);
                    formatted.push_str(" (json)")
                },
                Matcher::Any => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str("(any)");
                },
                Matcher::Missing => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str("(missing)");
                },
                Matcher::AnyOf(..) => {
                    formatted.push_str(key);
                    formatted.push_str(": ");
                    formatted.push_str("(any of)");
                },
            }

            formatted.push_str("\r\n");
        }

        match self.body {
            Matcher::Exact(ref value) | Matcher::JsonString(ref value) | Matcher::Regex(ref value) => {
                formatted.push_str(value);
                formatted.push_str("\r\n");
            },
            Matcher::Json(ref json_obj) => {
                formatted.push_str(&json_obj.to_string());
                formatted.push_str("\r\n")
            },
            Matcher::Missing => formatted.push_str("(missing)\r\n"),
            Matcher::AnyOf(..) => formatted.push_str("(any of)\r\n"),
            Matcher::Any => {}
        }

        f.write_str(&formatted)
    }
}
