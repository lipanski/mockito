use crate::diff;
use crate::matcher::{Matcher, PathAndQueryMatcher};
use crate::response::{Body, Response};
use crate::server::RemoteMock;
use crate::server::State;
use crate::Request;
use crate::{Error, ErrorKind};
use hyper::StatusCode;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::convert::Into;
use std::fmt;
use std::io;
use std::ops::Drop;
use std::path::Path;
use std::string::ToString;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone, Debug)]
pub struct InnerMock {
    pub(crate) id: String,
    pub(crate) method: String,
    pub(crate) path: PathAndQueryMatcher,
    pub(crate) headers: Vec<(String, Matcher)>,
    pub(crate) body: Matcher,
    pub(crate) response: Response,
    pub(crate) hits: usize,
    pub(crate) expected_hits_at_least: Option<usize>,
    pub(crate) expected_hits_at_most: Option<usize>,
}

impl fmt::Display for InnerMock {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatted = String::new();

        formatted.push_str("\r\n");
        formatted.push_str(&self.method);
        formatted.push(' ');
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
                formatted.push('=');
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

impl PartialEq for InnerMock {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.method == other.method
            && self.path == other.path
            && self.headers == other.headers
            && self.body == other.body
            && self.response == other.response
            && self.hits == other.hits
    }
}

///
/// Stores information about a mocked request. Should be initialized via `Server::mock()`.
///
#[derive(Debug)]
pub struct Mock {
    state: Arc<RwLock<State>>,
    inner: InnerMock,
    /// Used to warn of mocks missing a `.create()` call. See issue #112
    created: bool,
}

impl Mock {
    pub(crate) fn new<P: Into<Matcher>>(state: Arc<RwLock<State>>, method: &str, path: P) -> Mock {
        let inner = InnerMock {
            id: thread_rng()
                .sample_iter(&Alphanumeric)
                .map(char::from)
                .take(24)
                .collect(),
            method: method.to_owned().to_uppercase(),
            path: PathAndQueryMatcher::Unified(path.into()),
            headers: Vec::new(),
            body: Matcher::Any,
            response: Response::default(),
            hits: 0,
            expected_hits_at_least: None,
            expected_hits_at_most: None,
        };

        Self {
            state,
            inner,
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
    /// use mockito::Matcher;
    ///
    /// let mut s = mockito::Server::new();
    ///
    /// // This will match requests containing the URL-encoded
    /// // query parameter `greeting=good%20day`
    /// s.mock("GET", "/test")
    ///   .match_query(Matcher::UrlEncoded("greeting".into(), "good day".into()))
    ///   .create();
    ///
    /// // This will match requests containing the URL-encoded
    /// // query parameters `hello=world` and `greeting=good%20day`
    /// s.mock("GET", "/test")
    ///   .match_query(Matcher::AllOf(vec![
    ///     Matcher::UrlEncoded("hello".into(), "world".into()),
    ///     Matcher::UrlEncoded("greeting".into(), "good day".into())
    ///   ]))
    ///   .create();
    ///
    /// // You can achieve similar results with the regex matcher
    /// s.mock("GET", "/test")
    ///   .match_query(Matcher::Regex("hello=world".into()))
    ///   .create();
    /// ```
    ///
    pub fn match_query<M: Into<Matcher>>(mut self, query: M) -> Self {
        let new_path = match &self.inner.path {
            PathAndQueryMatcher::Unified(matcher) => {
                PathAndQueryMatcher::Split(Box::new(matcher.clone()), Box::new(query.into()))
            }
            PathAndQueryMatcher::Split(path, _) => {
                PathAndQueryMatcher::Split(path.clone(), Box::new(query.into()))
            }
        };

        self.inner.path = new_path;

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
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").match_header("content-type", "application/json");
    /// ```
    ///
    /// Like most other `Mock` methods, it allows chanining:
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/")
    ///   .match_header("content-type", "application/json")
    ///   .match_header("authorization", "password");
    /// ```
    ///
    pub fn match_header<M: Into<Matcher>>(mut self, field: &str, value: M) -> Self {
        self.inner
            .headers
            .push((field.to_owned().to_lowercase(), value.into()));

        self
    }

    ///
    /// Allows matching a particular request body when responding with a mock.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("POST", "/").match_body(r#"{"hello": "world"}"#).with_body("json").create();
    /// s.mock("POST", "/").match_body("hello=world").with_body("form").create();
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
    /// s.mock("POST", "/").match_body(tmp_file.as_path()).create();
    /// s.mock("POST", "/").match_body(random_bytes).create();
    /// s.mock("POST", "/").match_body(&mut f_read).create();
    /// ```
    ///
    pub fn match_body<M: Into<Matcher>>(mut self, body: M) -> Self {
        self.inner.body = body.into();

        self
    }

    ///
    /// Sets the status code of the mock response. The default status code is 200.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_status(201);
    /// ```
    ///
    #[track_caller]
    pub fn with_status(mut self, status: usize) -> Self {
        self.inner.response.status = StatusCode::from_u16(status as u16)
            .map_err(|_| Error::new_with_context(ErrorKind::InvalidStatusCode, status))
            .unwrap();

        self
    }

    ///
    /// Sets a header of the mock response.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_header("content-type", "application/json");
    /// ```
    ///
    pub fn with_header(mut self, field: &str, value: &str) -> Self {
        self.inner
            .response
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
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_body("hello world");
    /// ```
    ///
    pub fn with_body<StrOrBytes: AsRef<[u8]>>(mut self, body: StrOrBytes) -> Self {
        self.inner.response.body = Body::Bytes(body.as_ref().to_owned());
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
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_chunked_body(|w| w.write_all(b"hello world"));
    /// ```
    ///
    pub fn with_chunked_body(
        mut self,
        callback: impl Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.inner.response.body = Body::FnWithWriter(Arc::new(callback));
        self
    }

    ///
    /// **DEPRECATED:** Replaced by `Mock::with_chunked_body`.
    ///
    #[deprecated(since = "1.0.0", note = "Use `Mock::with_chunked_body` instead")]
    pub fn with_body_from_fn(
        self,
        callback: impl Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.with_chunked_body(callback)
    }

    ///
    /// Sets the body of the mock response dynamically while exposing the request object.
    ///
    /// You can use this method to provide a custom reponse body for every incoming request.
    ///
    /// The function must be thread-safe. If it's a closure, it can't be borrowing its context.
    /// Use `move` closures and `Arc` to share any data.
    ///
    /// ### Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// let _m = s.mock("GET", mockito::Matcher::Any).with_body_from_request(|request| {
    ///     if request.path() == "/bob" {
    ///         "hello bob".into()
    ///     } else if request.path() == "/alice" {
    ///         "hello alice".into()
    ///     } else {
    ///         "hello world".into()
    ///     }
    /// });
    /// ```
    ///
    pub fn with_body_from_request(
        mut self,
        callback: impl Fn(&Request) -> Vec<u8> + Send + Sync + 'static,
    ) -> Self {
        self.inner.response.body = Body::FnWithRequest(Arc::new(callback));
        self
    }

    ///
    /// Sets the body of the mock response from the contents of a file stored under `path`.
    /// Its `Content-Length` is handled automatically.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_body_from_file("tests/files/simple.http");
    /// ```
    ///
    #[track_caller]
    pub fn with_body_from_file(mut self, path: impl AsRef<Path>) -> Self {
        self.inner.response.body = Body::Bytes(
            std::fs::read(path)
                .map_err(|_| Error::new(ErrorKind::FileNotFound))
                .unwrap(),
        );
        self
    }

    ///
    /// Sets the expected amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    /// Defaults to 1 request.
    ///
    #[allow(clippy::missing_const_for_fn)]
    pub fn expect(mut self, hits: usize) -> Self {
        self.inner.expected_hits_at_least = Some(hits);
        self.inner.expected_hits_at_most = Some(hits);
        self
    }

    ///
    /// Sets the minimum amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    ///
    pub fn expect_at_least(mut self, hits: usize) -> Self {
        self.inner.expected_hits_at_least = Some(hits);
        if self.inner.expected_hits_at_most.is_some()
            && self.inner.expected_hits_at_most < self.inner.expected_hits_at_least
        {
            self.inner.expected_hits_at_most = None;
        }
        self
    }

    ///
    /// Sets the maximum amount of requests that this mock is supposed to receive.
    /// This is only enforced when calling the `assert` method.
    ///
    pub fn expect_at_most(mut self, hits: usize) -> Self {
        self.inner.expected_hits_at_most = Some(hits);
        if self.inner.expected_hits_at_least.is_some()
            && self.inner.expected_hits_at_least > self.inner.expected_hits_at_most
        {
            self.inner.expected_hits_at_least = None;
        }
        self
    }

    ///
    /// Asserts that the expected amount of requests (defaults to 1 request) were performed.
    ///
    #[track_caller]
    pub fn assert(&self) {
        let mutex = self.state.clone();
        let state = mutex.read().unwrap();
        if let Some(hits) = state.get_mock_hits(self.inner.id.clone()) {
            let matched = self.matched_hits(hits);
            let message = if !matched {
                let last_request = state.get_last_unmatched_request();
                self.build_assert_message(hits, last_request)
            } else {
                String::default()
            };

            assert!(matched, "{}", message)
        } else {
            panic!("could not retrieve enough information about the remote mock")
        }
    }

    ///
    /// Same as `Mock::assert` but async.
    ///
    pub async fn assert_async(&self) {
        let mutex = self.state.clone();
        let state = mutex.read().unwrap();
        if let Some(hits) = state.get_mock_hits(self.inner.id.clone()) {
            let matched = self.matched_hits(hits);
            let message = if !matched {
                let last_request = state.get_last_unmatched_request();
                self.build_assert_message(hits, last_request)
            } else {
                String::default()
            };

            assert!(matched, "{}", message)
        } else {
            panic!("could not retrieve enough information about the remote mock")
        }
    }

    ///
    /// Returns whether the expected amount of requests (defaults to 1) were performed.
    ///
    pub fn matched(&self) -> bool {
        let mutex = self.state.clone();
        let state = mutex.read().unwrap();
        let Some(hits) = state.get_mock_hits(self.inner.id.clone()) else {
            return false;
        };

        self.matched_hits(hits)
    }

    ///
    /// Same as `Mock::matched` but async.
    ///
    pub async fn matched_async(&self) -> bool {
        let mutex = self.state.clone();
        let state = mutex.read().unwrap();
        let Some(hits) = state.get_mock_hits(self.inner.id.clone()) else {
            return false;
        };

        self.matched_hits(hits)
    }

    ///
    /// Registers the mock to the server - your mock will be served only after calling this method.
    ///
    /// ## Example
    ///
    /// ```
    /// let mut s = mockito::Server::new();
    ///
    /// s.mock("GET", "/").with_body("hello world").create();
    /// ```
    ///
    pub fn create(mut self) -> Mock {
        let remote_mock = RemoteMock::new(self.inner.clone());
        let state = self.state.clone();
        let mut state = state.write().unwrap();
        state.mocks.push(remote_mock);

        self.created = true;

        self
    }

    ///
    /// Same as `Mock::create` but async.
    ///
    pub async fn create_async(mut self) -> Mock {
        let remote_mock = RemoteMock::new(self.inner.clone());
        let state = self.state.clone();
        let mut state = state.write().unwrap();
        state.mocks.push(remote_mock);

        self.created = true;

        self
    }

    ///
    /// Removes the mock from the server.
    ///
    pub fn remove(&self) {
        let mutex = self.state.clone();
        let mut state = mutex.write().unwrap();
        state.remove_mock(self.inner.id.clone());
    }

    ///
    /// Same as `Mock::remove` but async.
    ///
    pub async fn remove_async(&self) {
        let mutex = self.state.clone();
        let mut state = mutex.write().unwrap();
        state.remove_mock(self.inner.id.clone());
    }

    fn matched_hits(&self, hits: usize) -> bool {
        match (
            self.inner.expected_hits_at_least,
            self.inner.expected_hits_at_most,
        ) {
            (Some(min), Some(max)) => hits >= min && hits <= max,
            (Some(min), None) => hits >= min,
            (None, Some(max)) => hits <= max,
            (None, None) => hits == 1,
        }
    }

    fn build_assert_message(&self, hits: usize, last_request: Option<String>) -> String {
        let mut message = match (
            self.inner.expected_hits_at_least,
            self.inner.expected_hits_at_most,
        ) {
            (Some(min), Some(max)) if min == max => format!(
                "\n> Expected {} request(s) to:\n{}\n...but received {}\n\n",
                min, self, hits
            ),
            (Some(min), Some(max)) => format!(
                "\n> Expected between {} and {} request(s) to:\n{}\n...but received {}\n\n",
                min, max, self, hits
            ),
            (Some(min), None) => format!(
                "\n> Expected at least {} request(s) to:\n{}\n...but received {}\n\n",
                min, self, hits
            ),
            (None, Some(max)) => format!(
                "\n> Expected at most {} request(s) to:\n{}\n...but received {}\n\n",
                max, self, hits
            ),
            (None, None) => format!(
                "\n> Expected 1 request(s) to:\n{}\n...but received {}\n\n",
                self, hits
            ),
        };

        if let Some(last_request) = last_request {
            message.push_str(&format!(
                "> The last unmatched request was:\n{}\n",
                last_request
            ));

            let difference = diff::compare(&self.to_string(), &last_request);
            message.push_str(&format!("> Difference:\n{}\n", difference));
        }

        message
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        if !self.created {
            log::warn!("Missing .create() call on mock {}", self);
        }
    }
}

impl fmt::Display for Mock {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatted = String::new();
        formatted.push_str(&self.inner.to_string());
        f.write_str(&formatted)
    }
}

impl PartialEq for Mock {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
