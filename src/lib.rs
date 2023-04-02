#![warn(missing_docs)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/lipanski/mockito/master/docs/logo-black-100.png"
)]

//!
//! Mockito is a library for **generating and delivering HTTP mocks** in Rust. You can use it for integration testing
//! or offline work. Mockito runs a local pool of HTTP servers which create, deliver and remove the mocks.
//!
//! # Features
//!
//! - Supports HTTP1/2
//! - Runs your tests in parallel
//! - Comes with a wide range of request matchers (Regex, JSON, query parameters etc.)
//! - Checks that a mock was called (spy)
//! - Mocks multiple hosts at the same time
//! - Exposes sync and async interfaces
//! - Prints out a colored diff of the last unmatched request in case of errors
//! - Simple, intuitive API
//! - An awesome logo
//!
//! # Getting Started
//!
//! Add `mockito` to your `Cargo.toml` and start mocking:
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!   #[test]
//!   fn test_something() {
//!     // Request a new server from the pool
//!     let mut server = mockito::Server::new();
//!
//!     // Use one of these addresses to configure your client
//!     let host = server.host_with_port();
//!     let url = server.url();
//!
//!     // Create a mock
//!     let mock = server.mock("GET", "/hello")
//!       .with_status(201)
//!       .with_header("content-type", "text/plain")
//!       .with_header("x-api-key", "1234")
//!       .with_body("world")
//!       .create();
//!
//!     // Any calls to GET /hello beyond this line will respond with 201, the
//!     // `content-type: text/plain` header and the body "world".
//!
//!     // You can use `Mock::assert` to verify that your mock was called
//!     // mock.assert();
//!   }
//! }
//! ```
//!
//! If `Mock::assert` fails, a colored diff of the last unmatched request is displayed:
//!
//! ![colored-diff.png](https://raw.githubusercontent.com/lipanski/mockito/master/docs/colored-diff.png)
//!
//! Use **matchers** to handle requests to the same endpoint in a different way:
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!   #[test]
//!   fn test_something() {
//!     let mut server = mockito::Server::new();
//!
//!     server.mock("GET", "/greetings")
//!       .match_header("content-type", "application/json")
//!       .match_body(mockito::Matcher::PartialJsonString(
//!           "{\"greeting\": \"hello\"}".to_string(),
//!       ))
//!       .with_body("hello json")
//!       .create();
//!
//!     server.mock("GET", "/greetings")
//!       .match_header("content-type", "application/text")
//!       .match_body(mockito::Matcher::Regex("greeting=hello".to_string()))
//!       .with_body("hello text")
//!       .create();
//!   }
//! }
//! ```
//!
//! Start **multiple servers** to simulate requests to different hosts:
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!   #[test]
//!   fn test_something() {
//!     let mut twitter = mockito::Server::new();
//!     let mut github = mockito::Server::new();
//!
//!     // These mocks will be available at `twitter.url()`
//!     let twitter_mock = twitter.mock("GET", "/api").create();
//!
//!     // These mocks will be available at `github.url()`
//!     let github_mock = github.mock("GET", "/api").create();
//!   }
//! }
//! ```
//!
//! Write **async** tests (make sure to use the `_async` methods!):
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!   #[tokio::test]
//!   async fn test_something() {
//!     let mut server = Server::new_async().await;
//!     let m1 = server.mock("GET", "/a").with_body("aaa").create_async().await;
//!     let m2 = server.mock("GET", "/b").with_body("bbb").create_async().await;
//!
//!     let (m1, m2) = futures::join!(m1, m2);
//!
//!     // You can use `Mock::assert_async` to verify that your mock was called
//!     // m1.assert_async().await;
//!     // m2.assert_async().await;
//!   }
//! }
//! ```
//!
//! # Lifetime
//!
//! A mock is available only throughout the lifetime of the server. Once the server goes
//! out of scope, all mocks defined on that server are removed:
//!
//! ```
//! let address;
//!
//! {
//!     let mut s = mockito::Server::new();
//!     address = s.host_with_port();
//!
//!     s.mock("GET", "/").with_body("hi").create();
//!
//!     // Requests to `address` will be responded with "hi" til here
//! }
//!
//! // Requests to `address` will fail as of this point
//! ```
//!
//! You can remove individual mocks earlier by calling `Mock::remove`.
//!
//! # Async
//!
//! Mockito comes with both a sync and an async interface.
//!
//! In order to write async tests, you'll need to use the `_async` methods:
//!
//! - `Server::new_async`
//! - `Mock::create_async`
//! - `Mock::assert_async`
//! - `Mock::matched_async`
//! - `Mock::remove_async`
//!
//! ...otherwise your tests will not compile and you'll see the following error:
//!
//! ```text
//! Cannot block the current thread from within a runtime.
//! This happens because a function attempted to block the current thread while the thread is being used to drive asynchronous tasks.
//! ```
//!
//! # Matchers
//!
//! Mockito can match your request by method, path, query, headers or body.
//!
//! Various matchers are provided by the `Matcher` type: exact (string, binary, JSON), partial (regular expressions,
//! JSON), any or missing. The following guide will walk you through the most common matchers. Check the
//! `Matcher` documentation for all the rest.
//!
//! # Matching by path and query
//!
//! By default, the request path and query is compared by its exact value:
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Matches only calls to GET /hello
//! s.mock("GET", "/hello").create();
//!
//! // Matches only calls to GET /hello?world=1
//! s.mock("GET", "/hello?world=1").create();
//! ```
//!
//! You can also match the path partially, by using a regular expression:
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Will match calls to GET /hello/1 and GET /hello/2
//! s.mock("GET",
//!     mockito::Matcher::Regex(r"^/hello/(1|2)$".to_string())
//!   ).create();
//! ```
//!
//! Or you can catch all requests, by using the `Matcher::Any` variant:
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Will match any GET request
//! s.mock("GET", mockito::Matcher::Any).create();
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
//! let mut s = mockito::Server::new();
//!
//! // This will match requests containing the URL-encoded
//! // query parameter `greeting=good%20day`
//! s.mock("GET", "/test")
//!   .match_query(mockito::Matcher::UrlEncoded("greeting".into(), "good day".into()))
//!   .create();
//!
//! // This will match requests containing the URL-encoded
//! // query parameters `hello=world` and `greeting=good%20day`
//! s.mock("GET", "/test")
//!   .match_query(mockito::Matcher::AllOf(vec![
//!     mockito::Matcher::UrlEncoded("hello".into(), "world".into()),
//!     mockito::Matcher::UrlEncoded("greeting".into(), "good day".into())
//!   ]))
//!   .create();
//!
//! // You can achieve similar results with the regex matcher
//! s.mock("GET", "/test")
//!   .match_query(mockito::Matcher::Regex("hello=world".into()))
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
//! let mut s = mockito::Server::new();
//!
//! // This will perform a full match against the query part
//! s.mock("GET", "/test?hello=world").create();
//! ```
//!
//! If you'd like to ignore the query entirely, use the `Matcher::Any` variant:
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // This will match requests to GET /test with any query
//! s.mock("GET", "/test").match_query(mockito::Matcher::Any).create();
//! ```
//!
//! # Matching by header
//!
//! By default, headers are compared by their exact value. The header name letter case is ignored though.
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! s.mock("GET", "/hello")
//!   .match_header("content-type", "application/json")
//!   .with_body(r#"{"hello": "world"}"#)
//!   .create();
//!
//! s.mock("GET", "/hello")
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
//! let mut s = mockito::Server::new();
//!
//! s.mock("GET", "/hello")
//!   .match_header("content-type", mockito::Matcher::Regex(r".*json.*".to_string()))
//!   .with_body(r#"{"hello": "world"}"#)
//!   .create();
//! ```
//!
//! Or you can match a header *only by its field name*, by setting the `Mock::match_header` value to `Matcher::Any`.
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! s.mock("GET", "/hello")
//!  .match_header("content-type", mockito::Matcher::Any)
//!  .with_body("something")
//!  .create();
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
//! let mut s = mockito::Server::new();
//!
//! s.mock("GET", "/hello")
//!   .match_header("authorization", mockito::Matcher::Missing)
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
//! let mut s = mockito::Server::new();
//!
//! // Will match requests to POST / whenever the request body is "hello"
//! s.mock("POST", "/").match_body("hello").create();
//! ```
//!
//! Or you can match the body by using a regular expression:
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Will match requests to POST / whenever the request body *contains* the word "hello" (e.g. "hello world")
//! s.mock("POST", "/").match_body(
//!     mockito::Matcher::Regex("hello".to_string())
//!   ).create();
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
//!
//! # fn main() {
//! let mut s = mockito::Server::new();
//! // Will match requests to POST / whenever the request body matches the json object
//! s.mock("POST", "/").match_body(mockito::Matcher::Json(json!({"hello": "world"}))).create();
//! # }
//! ```
//!
//! If `serde_json::json!` is not exposed, you can use `Matcher::JsonString` the same way,
//! but by passing a `String` to the matcher:
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Will match requests to POST / whenever the request body matches the json object
//! s.mock("POST", "/")
//!     .match_body(
//!        mockito::Matcher::JsonString(r#"{"hello": "world"}"#.to_string())
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
//! let mut s = mockito::Server::new();
//!
//! // Will match requests to POST / whenever the request body is either `hello=world` or `{"hello":"world"}`
//! s.mock("POST", "/")
//!     .match_body(
//!         mockito::Matcher::AnyOf(vec![
//!             mockito::Matcher::Exact("hello=world".to_string()),
//!             mockito::Matcher::JsonString(r#"{"hello": "world"}"#.to_string()),
//!         ])
//!      )
//!     .create();
//! ```
//!
//! # The `AllOf` matcher
//!
//! The `Matcher::AllOf` construct takes a vector of matchers as arguments and will be enabled
//! if all of the provided matchers match the request.
//!
//! ## Example
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! // Will match requests to POST / whenever the request body contains both `hello` and `world`
//! s.mock("POST", "/")
//!     .match_body(
//!         mockito::Matcher::AllOf(vec![
//!             mockito::Matcher::Regex("hello".to_string()),
//!             mockito::Matcher::Regex("world".to_string()),
//!         ])
//!      )
//!     .create();
//! ```
//!
//! # Asserts
//!
//! You can use the `Mock::assert` method to **assert that a mock was called**. In other words,
//! `Mock#assert` can validate that your code performed the expected HTTP request.
//!
//! By default, the method expects only **one** request to your mock.
//!
//! ## Example
//!
//! ```no_run
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//!
//! let mut s = mockito::Server::new();
//! let mock = s.mock("GET", "/hello").create();
//!
//! {
//!     // Place a request
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
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
//!
//! let mut s = mockito::Server::new();
//! let english_hello_mock = s.mock("GET", "/hello").with_body("good bye").create();
//! let french_hello_mock = s.mock("GET", "/hello").with_body("au revoir").create();
//!
//! {
//!     // Place a request to GET /hello
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
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
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
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
//!
//! let mut s = mockito::Server::new();
//!
//! let mock = s.mock("GET", "/hello").expect(3).create();
//!
//! for _ in 0..3 {
//!     // Place a request
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
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
//!
//! let mut s = mockito::Server::new();
//!
//! let mock = s.mock("GET", "/hello").expect_at_least(2).expect_at_most(4).create();
//!
//! for _ in 0..3 {
//!     // Place a request
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
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
//! a missing or incomplete mock. A colored diff is also displayed:
//!
//! ![colored-diff.png](https://raw.githubusercontent.com/lipanski/mockito/master/docs/colored-diff.png)
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
//!
//! let mut s = mockito::Server::new();
//!
//! let mock = s.mock("GET", "/").create();
//!
//! {
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
//!     stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//!
//! assert!(mock.matched());
//!
//! {
//!     let mut stream = TcpStream::connect(s.host_with_port()).unwrap();
//!     stream.write_all("GET / HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
//!     let mut response = String::new();
//!     stream.read_to_string(&mut response).unwrap();
//!     stream.flush().unwrap();
//! }
//! assert!(!mock.matched());
//! ```
//!
//! # Non-matching calls
//!
//! Any calls to the Mockito server that are not matched will return *501 Mock Not Found*.
//!
//! Note that **mocks are matched in reverse order** - the most recent one wins.
//!
//! # Cleaning up
//!
//! As mentioned earlier, mocks are cleaned up whenever the server goes out of scope. If you
//! need to remove them earlier, you can call `Server::reset` to remove all mocks registered
//! so far:
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! s.mock("GET", "/1").create();
//! s.mock("GET", "/2").create();
//! s.mock("GET", "/3").create();
//!
//! s.reset();
//!
//! // Nothing is mocked at this point
//! ```
//!
//! ...or you can call `Mock::remove` to remove a single mock:
//!
//! ```
//! let mut s = mockito::Server::new();
//!
//! let m1 = s.mock("GET", "/1").create();
//! let m2 = s.mock("GET", "/2").create();
//!
//! m1.remove();
//!
//! // Only m2 is available at this point
//! ```
//!
//! # Debug
//!
//! Mockito uses the `env_logger` crate under the hood to provide useful debugging information.
//!
//! If you'd like to activate the debug output, introduce the [env_logger](https://crates.rs/crates/env_logger) crate
//! to your project and initialize it before each test that needs debugging:
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
pub use error::{Error, ErrorKind};
#[allow(deprecated)]
pub use matcher::Matcher;
pub use mock::Mock;
pub use request::Request;
pub use server::Server;
pub use server_pool::ServerGuard;

mod diff;
mod error;
mod matcher;
mod mock;
mod request;
mod response;
mod server;
mod server_pool;
