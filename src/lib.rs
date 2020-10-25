#![warn(missing_docs)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/lipanski/mockito/master/docs/logo-black.png"
)]

//!
//! Mockito is a library for creating HTTP mocks to be used in integration tests or for offline work.
//! It runs an HTTP server on a local port which delivers, creates and remove the mocks.
//!
//! The server is run on a separate thread within the same process and will be removed
//! at the end of the run.
//!
//! # Getting Started
//!
//! Use `mockito::server_url()` or `mockito::server_address()` as the base URL for any mocked
//! client in your tests. One way to do this is by using compiler flags:
//!
//! ## Example
//!
//! ```
//! #[cfg(test)]
//! use mockito;
//!
//! fn main() {
//!   #[cfg(not(test))]
//!   let url = "https://api.twitter.com";
//!
//!   #[cfg(test)]
//!   let url = &mockito::server_url();
//!
//!   // Use url as the base URL for your client
//! }
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
//! use mockito::{mock, server_address};
//!
//! let mock = mock("GET", "/hello").create();
//!
//! {
//!     // Place a request
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! mock.assert();
//! ```
//!
//! When several mocks can match a request, Mockito applies the first one that still expects requests.
//! You can use this behaviour to provide **different responses for subsequent requests to the same endpoint**.
//!
//! ## Example
//!
//! ```
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, server_address};
//!
//! let english_hello_mock = mock("GET", "/hello").with_body("good bye").create();
//! let french_hello_mock = mock("GET", "/hello").with_body("au revoir").create();
//!
//! {
//!     // Place a request to GET /hello
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! english_hello_mock.assert();
//!
//! {
//!     // Place another request to GET /hello
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! french_hello_mock.assert();
//! ```
//!
//! If you're expecting more than 1 request, you can use the `Mock::expect` method to specify the exact amount of requests:
//!
//! ## Example
//!
//! ```no_run
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, server_address};
//!
//! let mock = mockito::mock("GET", "/hello").expect(3).create();
//!
//! for _ in 0..3 {
//!     // Place a request
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! mock.assert();
//! ```
//!
//! You can also work with ranges, by using the `Mock::expect_at_least` and `Mock::expect_at_most` methods:
//!
//! ## Example
//!
//! ```no_run
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, server_address};
//!
//! let mock = mockito::mock("GET", "/hello").expect_at_least(2).expect_at_most(4).create();
//!
//! for _ in 0..3 {
//!     // Place a request
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
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
//! a missing or incomplete mock. A colored diff is also displayed.
//!
//! Color output is enabled by default, but can be toggled with the `color` feature flag.
//!
//! Here's an example of how a `Mock#assert` error looks like:
//!
//! ```text
//! > Expected 1 request(s) to:
//!
//! POST /users?number=one
//! bob
//!
//! ...but received 0
//!
//! > The last unmatched request was:
//!
//! POST /users?number=two
//! content-length: 5
//! alice
//!
//! > Difference:
//!
//! # A colored diff
//!
//! ```
//!
//! You can also use the `matched` method to return a boolean for whether the mock was called the
//! correct number of times without panicking
//!
//! ## Example
//!
//! ```
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//! use mockito::{mock, server_address};
//!
//! let mock = mock("GET", "/").create();
//!
//! {
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! assert!(mock.matched());
//!
//! {
//!     let mut stream = TcpStream::connect(server_address()).unwrap();
//!     stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//! assert!(!mock.matched());
//! ```
//!
//! # Matchers
//!
//! Mockito can match your request by method, path, query, headers or body.
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
//! # Matching by query
//!
//! You can match the query part by using the `Mock#match_query` function together with the various matchers,
//! most notably `Matcher::UrlEncoded`:
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // This will match requests containing the URL-encoded
//! // query parameter `greeting=good%20day`
//! let _m1 = mock("GET", "/test")
//!   .match_query(Matcher::UrlEncoded("greeting".into(), "good day".into()))
//!   .create();
//!
//! // This will match requests containing the URL-encoded
//! // query parameters `hello=world` and `greeting=good%20day`
//! let _m2 = mock("GET", "/test")
//!   .match_query(Matcher::AllOf(vec![
//!     Matcher::UrlEncoded("hello".into(), "world".into()),
//!     Matcher::UrlEncoded("greeting".into(), "good day".into())
//!   ]))
//!   .create();
//!
//! // You can achieve similar results with the regex matcher
//! let _m3 = mock("GET", "/test")
//!   .match_query(Matcher::Regex("hello=world".into()))
//!   .create();
//! ```
//!
//! Note that the key/value arguments for `Matcher::UrlEncoded` should be left in plain (unencoded) format.
//!
//! You can also specify the query as part of the path argument in a `mock` call, in which case an exact
//! match will be performed:
//!
//! ## Example
//!
//! ```
//! use mockito::mock;
//!
//! // This will perform a full match against the query part
//! let _m = mock("GET", "/test?hello=world").create();
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
//!   .with_body(r#"{"hello": "world"}"#)
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
//!   .with_body(r#"{"hello": "world"}"#)
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
//! let _m = mock("POST", "/").match_body(Matcher::Json(json!({"hello": "world"}))).create();
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
//!        Matcher::JsonString(r#"{"hello": "world"}"#.to_string())
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
//!             Matcher::JsonString(r#"{"hello": "world"}"#.to_string()),
//!         ])
//!      )
//!     .create();
//!```
//!
//! # The `AllOf` matcher
//!
//! The `Matcher::AllOf` construct takes a vector of matchers as arguments and will be enabled
//! if all of the provided matchers match the request.
//!
//! ## Example
//!
//! ```
//! use mockito::{mock, Matcher};
//!
//! // Will match requests to POST / whenever the request body contains both `hello` and `world`
//! let _m = mock("POST", "/")
//!     .match_body(
//!         Matcher::AllOf(vec![
//!             Matcher::Regex("hello".to_string()),
//!             Matcher::Regex("world".to_string()),
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
//! # Debug
//!
//! Mockito uses the `env_logger` crate under the hood to provide useful debugging information.
//!
//! If you'd like to activate the debug output, introduce the [env_logger](https://crates.rs/crates/env_logger) crate
//! within your project and initialize it before each test that needs debugging:
//!
//! ```
//! #[test]
//! fn example_test() {
//!     let _ = env_logger::try_init();
//!     // ...
//! }
//! ```
//!
//! Run your tests with:
//!
//! ```sh
//! RUST_LOG=mockito=debug cargo test
//! ```
//!
//! # Threads
//!
//! Mockito records all your mocks on the same server running in the background. For this
//! reason, Mockito tests are run sequentially. This is handled internally via a thread-local
//! mutex lock acquired **whenever you create a mock**. Tests that don't create mocks will
//! still be run in parallel.
//!

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod diff;
mod request;
mod response;
mod server;

type Request = request::Request;
type Response = response::Response;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{From, Into};
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::ops::Drop;
use std::path::Path;
use std::string::ToString;
use std::sync::Arc;
use std::sync::{LockResult, Mutex, MutexGuard};

lazy_static! {
    // A global lock that ensure all Mockito tests are run on a single thread.
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

thread_local!(
    // A thread-local reference to the global lock. This is acquired within `Mock#create()`.
    static LOCAL_TEST_MUTEX: RefCell<LockResult<MutexGuard<'static, ()>>> =
        RefCell::new(TEST_MUTEX.lock());
);

///
/// Points to the address the mock server is running at.
/// Can be used with `std::net::TcpStream`.
///
#[deprecated(note = "Call server_address() instead")]
pub const SERVER_ADDRESS: &str = SERVER_ADDRESS_INTERNAL;
const SERVER_ADDRESS_INTERNAL: &str = "127.0.0.1:1234";

///
/// Points to the URL the mock server is running at.
///
#[deprecated(note = "Call server_url() instead")]
pub const SERVER_URL: &str = "http://127.0.0.1:1234";

pub use crate::server::address as server_address;
pub use crate::server::url as server_url;
use assert_json_diff::assert_json_include_no_panic;

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

    let mut state = server::STATE.lock().unwrap();
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
    /// Matches the body content as a binary file
    Binary(BinaryBody),
    /// Matches a path or header value by a regular expression.
    Regex(String),
    /// Matches a specified JSON body from a `serde_json::Value`
    Json(serde_json::Value),
    /// Matches a specified JSON body from a `String`
    JsonString(String),
    /// Matches a partial JSON body from a `serde_json::Value`
    PartialJson(serde_json::Value),
    /// Matches a specified partial JSON body from a `String`
    PartialJsonString(String),
    /// Matches a URL-encoded key/value pair, where both key and value should be specified
    /// in plain (unencoded) format
    UrlEncoded(String, String),
    /// At least one matcher must match
    AnyOf(Vec<Matcher>),
    /// All matchers must match
    AllOf(Vec<Matcher>),
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

#[allow(clippy::fallible_impl_from)]
impl From<&Path> for Matcher {
    fn from(value: &Path) -> Self {
        // We want the code to panic if the path is not readable.
        Matcher::Binary(BinaryBody::from_path(value).unwrap())
    }
}

impl From<&mut File> for Matcher {
    fn from(value: &mut File) -> Self {
        Matcher::Binary(BinaryBody::from_file(value))
    }
}

impl From<Vec<u8>> for Matcher {
    fn from(value: Vec<u8>) -> Self {
        Matcher::Binary(BinaryBody::from_bytes(value))
    }
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let join_matches = |matches: &[Self]| {
            matches
                .iter()
                .map(Self::to_string)
                .fold(String::new(), |acc, matcher| {
                    if acc.is_empty() {
                        matcher
                    } else {
                        format!("{}, {}", acc, matcher)
                    }
                })
        };

        let result = match self {
            Matcher::Exact(ref value) => value.to_string(),
            Matcher::Binary(ref file) => format!("{} (binary)", file),
            Matcher::Regex(ref value) => format!("{} (regex)", value),
            Matcher::Json(ref json_obj) => format!("{} (json)", json_obj),
            Matcher::JsonString(ref value) => format!("{} (json)", value),
            Matcher::PartialJson(ref json_obj) => format!("{} (partial json)", json_obj),
            Matcher::PartialJsonString(ref value) => format!("{} (partial json)", value),
            Matcher::UrlEncoded(ref field, ref value) => format!("{}={} (urlencoded)", field, value),
            Matcher::Any => "(any)".to_string(),
            Matcher::AnyOf(x) => format!("({}) (any of)", join_matches(x)),
            Matcher::AllOf(x) => format!("({}) (all of)", join_matches(x)),
            Matcher::Missing => "(missing)".to_string(),
        };
        write!(f, "{}", result)
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
            }
            Matcher::AllOf(ref matchers) if header_values.is_empty() => {
                matchers.iter().all(|m| m.matches_values(header_values))
            }
            _ => {
                !header_values.is_empty() && header_values.iter().all(|val| self.matches_value(val))
            }
        }
    }

    fn matches_binary_value(&self, binary: &[u8]) -> bool {
        match self {
            Matcher::Binary(ref file) => binary == &*file.content,
            _ => false,
        }
    }

    #[allow(deprecated)]
    fn matches_value(&self, other: &str) -> bool {
        match self {
            Matcher::Exact(ref value) => value == other,
            Matcher::Binary(_) => false,
            Matcher::Regex(ref regex) => Regex::new(regex).unwrap().is_match(other),
            Matcher::Json(ref json_obj) => {
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                *json_obj == other
            }
            Matcher::JsonString(ref value) => {
                let value: serde_json::Value = serde_json::from_str(value).unwrap();
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                value == other
            }
            Matcher::PartialJson(ref json_obj) => {
                let actual: serde_json::Value = serde_json::from_str(other).unwrap();
                let expected = json_obj.clone();
                assert_json_include_no_panic(&actual, &expected).is_ok()
            }
            Matcher::PartialJsonString(ref value) => {
                let expected: serde_json::Value = serde_json::from_str(value).unwrap();
                let actual: serde_json::Value = serde_json::from_str(other).unwrap();
                assert_json_include_no_panic(&actual, &expected).is_ok()
            }
            Matcher::UrlEncoded(ref expected_field, ref expected_value) => {
                serde_urlencoded::from_str::<HashMap<String, String>>(other)
                    .map(|params: HashMap<_, _>| {
                        params.into_iter().any(|(ref field, ref value)| {
                            field == expected_field && value == expected_value
                        })
                    })
                    .unwrap_or(false)
            }
            Matcher::Any => true,
            Matcher::AnyOf(ref matchers) => matchers.iter().any(|m| m.matches_value(other)),
            Matcher::AllOf(ref matchers) => matchers.iter().all(|m| m.matches_value(other)),
            Matcher::Missing => other.is_empty(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum PathAndQueryMatcher {
    Unified(Matcher),
    Split(Box<Matcher>, Box<Matcher>),
}

impl PathAndQueryMatcher {
    fn matches_value(&self, other: &str) -> bool {
        match self {
            PathAndQueryMatcher::Unified(matcher) => matcher.matches_value(other),
            PathAndQueryMatcher::Split(ref path_matcher, ref query_matcher) => {
                let mut parts = other.splitn(2, '?');
                let path = parts.next().unwrap();
                let query = parts.next().unwrap_or("");

                path_matcher.matches_value(path) && query_matcher.matches_value(query)
            }
        }
    }
}

///
/// Represents a binary object the body should be matched against
///
#[derive(Debug, Clone)]
pub struct BinaryBody {
    path: Option<String>,
    content: Vec<u8>,
}

impl BinaryBody {
    /// Read the content from path and initialize a `BinaryBody`
    ///
    /// # Errors
    ///
    /// The same resulting from a failed `std::fs::read`.
    pub fn from_path(path: &Path) -> Result<Self, io::Error> {
        Ok(Self {
            path: path.to_str().map(ToString::to_string),
            content: std::fs::read(path)?,
        })
    }

    /// Read the content from a &mut File and initialize a `BinaryBody`
    pub fn from_file(file: &mut File) -> Self {
        Self {
            path: None,
            content: get_content_from(file),
        }
    }

    /// Instantiate the matcher directly passing the content
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_bytes(content: Vec<u8>) -> Self {
        Self {
            path: None,
            content,
        }
    }
}

fn get_content_from(file: &mut File) -> Vec<u8> {
    let mut filecontent: Vec<u8> = Vec::new();
    file.read_to_end(&mut filecontent).unwrap();
    filecontent
}

impl PartialEq for BinaryBody {
    fn eq(&self, other: &Self) -> bool {
        match (self.path.as_ref(), other.path.as_ref()) {
            (Some(p), Some(o)) => p == o,
            _ => self.content == other.content,
        }
    }
}

impl fmt::Display for BinaryBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(filepath) = self.path.as_ref() {
            write!(f, "filepath: {}", filepath)
        } else {
            let len: usize = std::cmp::min(self.content.len(), 8);
            let first_bytes: Vec<u8> = self.content.to_owned().into_iter().take(len).collect();
            write!(f, "filecontent: {:?}", first_bytes)
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
    path: PathAndQueryMatcher,
    headers: Vec<(String, Matcher)>,
    body: Matcher,
    response: Response,
    hits: usize,
    expected_hits_at_least: Option<usize>,
    expected_hits_at_most: Option<usize>,
    is_remote: bool,

    /// Used to warn of mocks missing a `.create()` call. See issue #112
    created: bool,
}

impl Mock {
    fn new<P: Into<Matcher>>(method: &str, path: P) -> Self {
        Self {
            id: thread_rng().sample_iter(&Alphanumeric).take(24).collect(),
            method: method.to_owned().to_uppercase(),
            path: PathAndQueryMatcher::Unified(path.into()),
            headers: Vec::new(),
            body: Matcher::Any,
            response: Response::default(),
            hits: 0,
            expected_hits_at_least: None,
            expected_hits_at_most: None,
            is_remote: false,
            created: false,
        }
    }

    ///
    /// Allows matching against the query part when responding with a mock.
    ///
    /// Note that you can also specify the query as part of the path argument
    /// in a `mock` call, in which case an exact match will be performed.
    /// Any future calls of `Mock#match_query` will override the query matcher.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::{mock, Matcher};
    ///
    /// // This will match requests containing the URL-encoded
    /// // query parameter `greeting=good%20day`
    /// let _m1 = mock("GET", "/test")
    ///   .match_query(Matcher::UrlEncoded("greeting".into(), "good day".into()))
    ///   .create();
    ///
    /// // This will match requests containing the URL-encoded
    /// // query parameters `hello=world` and `greeting=good%20day`
    /// let _m2 = mock("GET", "/test")
    ///   .match_query(Matcher::AllOf(vec![
    ///     Matcher::UrlEncoded("hello".into(), "world".into()),
    ///     Matcher::UrlEncoded("greeting".into(), "good day".into())
    ///   ]))
    ///   .create();
    ///
    /// // You can achieve similar results with the regex matcher
    /// let _m3 = mock("GET", "/test")
    ///   .match_query(Matcher::Regex("hello=world".into()))
    ///   .create();
    /// ```
    ///
    pub fn match_query<M: Into<Matcher>>(mut self, query: M) -> Self {
        let new_path = match &self.path {
            PathAndQueryMatcher::Unified(matcher) => {
                PathAndQueryMatcher::Split(Box::new(matcher.clone()), Box::new(query.into()))
            }
            PathAndQueryMatcher::Split(path, _) => {
                PathAndQueryMatcher::Split(path.clone(), Box::new(query.into()))
            }
        };

        self.path = new_path;

        self
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
        self.headers
            .push((field.to_owned().to_lowercase(), value.into()));

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
    /// let _m1 = mock("POST", "/").match_body(r#"{"hello": "world"}"#).with_body("json").create();
    /// let _m2 = mock("POST", "/").match_body("hello=world").with_body("form").create();
    ///
    /// // Requests passing `{"hello": "world"}` inside the body will be responded with "json".
    /// // Requests passing `hello=world` inside the body will be responded with "form".
    ///
    /// // Create a temporary file
    /// use std::env;
    /// use std::fs::File;
    /// use std::io::Write;
    /// use std::path::Path;
    /// use rand;
    /// use rand::Rng;
    ///
    /// let random_bytes: Vec<u8> = (0..1024).map(|_| rand::random::<u8>()).collect();
    ///
    /// let mut tmp_file = env::temp_dir();
    /// tmp_file.push("test_file.txt");
    /// let mut f_write = File::create(tmp_file.clone()).unwrap();
    /// f_write.write_all(random_bytes.as_slice()).unwrap();
    /// let mut f_read = File::open(tmp_file.clone()).unwrap();
    ///
    ///
    /// // the following are equivalent ways of defining a mock matching
    /// // a binary payload
    /// let _b1 = mock("POST", "/").match_body(tmp_file.as_path()).create();
    /// let _b3 = mock("POST", "/").match_body(random_bytes).create();
    /// let _b2 = mock("POST", "/").match_body(&mut f_read).create();
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
        self.response
            .headers
            .push((field.to_owned(), value.to_owned()));

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
        self.response.body = response::Body::Bytes(body.as_ref().to_owned());
        self
    }

    ///
    /// Sets the body of the mock response dynamically. The response will use chunked transfer encoding.
    ///
    /// The function must be thread-safe. If it's a closure, it can't be borrowing its context.
    /// Use `move` closures and `Arc` to share any data.
    ///
    /// ## Example
    ///
    /// ```
    /// use mockito::mock;
    ///
    /// let _m = mock("GET", "/").with_body_from_fn(|w| w.write_all(b"hello world"));
    /// ```
    ///
    pub fn with_body_from_fn(
        mut self,
        cb: impl Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.response.body = response::Body::Fn(Arc::new(cb));
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
    pub fn with_body_from_file(mut self, path: impl AsRef<Path>) -> Self {
        self.response.body = response::Body::Bytes(std::fs::read(path).unwrap());
        self
    }

    ///
    /// Sets the expected amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    /// Defaults to 1 request.
    ///
    #[allow(clippy::missing_const_for_fn)]
    pub fn expect(mut self, hits: usize) -> Self {
        self.expected_hits_at_least = Some(hits);
        self.expected_hits_at_most = Some(hits);
        self
    }

    ///
    /// Sets the minimum amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    ///
    pub fn expect_at_least(mut self, hits: usize) -> Self {
        self.expected_hits_at_least = Some(hits);
        if self.expected_hits_at_most.is_some()
            && self.expected_hits_at_most < self.expected_hits_at_least
        {
            self.expected_hits_at_most = None;
        }
        self
    }

    ///
    /// Sets the maximum amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    ///
    pub fn expect_at_most(mut self, hits: usize) -> Self {
        self.expected_hits_at_most = Some(hits);
        if self.expected_hits_at_least.is_some()
            && self.expected_hits_at_least > self.expected_hits_at_most
        {
            self.expected_hits_at_least = None;
        }
        self
    }

    ///
    /// Asserts that the expected amount of requests (defaults to 1 request) were performed.
    ///
    pub fn assert(&self) {
        let mut opt_message = None;

        {
            let state = server::STATE.lock().unwrap();

            if let Some(remote_mock) = state.mocks.iter().find(|mock| mock.id == self.id) {
                let mut message = match (self.expected_hits_at_least, self.expected_hits_at_most) {
                    (Some(min), Some(max)) if min == max => format!(
                        "\n> Expected {} request(s) to:\n{}\n...but received {}\n\n",
                        min, self, remote_mock.hits
                    ),
                    (Some(min), Some(max)) => format!(
                        "\n> Expected between {} and {} request(s) to:\n{}\n...but received {}\n\n",
                        min, max, self, remote_mock.hits
                    ),
                    (Some(min), None) => format!(
                        "\n> Expected at least {} request(s) to:\n{}\n...but received {}\n\n",
                        min, self, remote_mock.hits
                    ),
                    (None, Some(max)) => format!(
                        "\n> Expected at most {} request(s) to:\n{}\n...but received {}\n\n",
                        max, self, remote_mock.hits
                    ),
                    (None, None) => format!(
                        "\n> Expected 1 request(s) to:\n{}\n...but received {}\n\n",
                        self, remote_mock.hits
                    ),
                };

                if let Some(last_request) = state.unmatched_requests.last() {
                    message.push_str(&format!(
                        "> The last unmatched request was:\n{}\n",
                        last_request
                    ));

                    let difference = diff::compare(&self.to_string(), &last_request.to_string());
                    message.push_str(&format!("> Difference:\n{}\n", difference));
                }

                opt_message = Some(message);
            }
        }

        if let Some(message) = opt_message {
            assert!(self.matched(), "{}", message)
        } else {
            panic!("Could not retrieve enough information about the remote mock.")
        }
    }

    ///
    /// Returns whether the expected amount of requests (defaults to 1) were performed.
    ///
    pub fn matched(&self) -> bool {
        let state = server::STATE.lock().unwrap();

        state
            .mocks
            .iter()
            .find(|mock| mock.id == self.id)
            .map_or(false, |remote_mock| {
                let hits = remote_mock.hits;

                match (self.expected_hits_at_least, self.expected_hits_at_most) {
                    (Some(min), Some(max)) => hits >= min && hits <= max,
                    (Some(min), None) => hits >= min,
                    (None, Some(max)) => hits <= max,
                    (None, None) => hits == 1,
                }
            })
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
    #[must_use]
    pub fn create(mut self) -> Self {
        server::try_start();

        // Ensures Mockito tests are run sequentially.
        LOCAL_TEST_MUTEX.with(|_| {});

        let mut state = server::STATE.lock().unwrap();

        self.created = true;

        let mut remote_mock = self.clone();
        remote_mock.is_remote = true;
        state.mocks.push(remote_mock);

        self
    }

    #[allow(clippy::missing_const_for_fn)]
    fn is_local(&self) -> bool {
        !self.is_remote
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        if self.is_local() {
            let mut state = server::STATE.lock().unwrap();

            if let Some(pos) = state.mocks.iter().position(|mock| mock.id == self.id) {
                state.mocks.remove(pos);
            }

            debug!("Mock::drop() called for {}", self);

            if !self.created {
                warn!("Missing .create() call on mock {}", self);
            }
        }
    }
}

impl fmt::Display for PathAndQueryMatcher {
    #[allow(deprecated)]
    #[allow(clippy::write_with_newline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathAndQueryMatcher::Unified(matcher) => write!(f, "{}\r\n", &matcher),
            PathAndQueryMatcher::Split(path, query) => write!(f, "{}?{}\r\n", &path, &query),
        }
    }
}

impl fmt::Display for Mock {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatted = String::new();

        formatted.push_str("\r\n");
        formatted.push_str(&self.method);
        formatted.push_str(" ");
        formatted.push_str(&self.path.to_string());

        for &(ref key, ref value) in &self.headers {
            formatted.push_str(key);
            formatted.push_str(": ");
            formatted.push_str(&value.to_string());
            formatted.push_str("\r\n");
        }

        match self.body {
            Matcher::Exact(ref value)
            | Matcher::JsonString(ref value)
            | Matcher::PartialJsonString(ref value)
            | Matcher::Regex(ref value) => {
                formatted.push_str(value);
                formatted.push_str("\r\n");
            }
            Matcher::Binary(_) => {
                formatted.push_str("(binary)\r\n");
            }
            Matcher::Json(ref json_obj) | Matcher::PartialJson(ref json_obj) => {
                formatted.push_str(&json_obj.to_string());
                formatted.push_str("\r\n")
            }
            Matcher::UrlEncoded(ref field, ref value) => {
                formatted.push_str(field);
                formatted.push_str("=");
                formatted.push_str(value);
            }
            Matcher::Missing => formatted.push_str("(missing)\r\n"),
            Matcher::AnyOf(..) => formatted.push_str("(any of)\r\n"),
            Matcher::AllOf(..) => formatted.push_str("(all of)\r\n"),
            Matcher::Any => {}
        }

        f.write_str(&formatted)
    }
}
