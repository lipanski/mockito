use crate::{Error, ErrorKind};
use hyper::body;
use hyper::body::Buf;
use hyper::Body as HyperBody;
use hyper::Request as HyperRequest;

///
/// Stores a HTTP request
///
#[derive(Debug)]
pub struct Request {
    inner: HyperRequest<HyperBody>,
    body: Option<Vec<u8>>,
}

impl Request {
    pub(crate) fn new(request: HyperRequest<HyperBody>) -> Self {
        Request {
            inner: request,
            body: None,
        }
    }

    /// The HTTP method
    pub fn method(&self) -> &str {
        self.inner.method().as_ref()
    }

    /// The path excluding the query part
    pub fn path(&self) -> &str {
        self.inner.uri().path()
    }

    /// The path including the query part
    pub fn path_and_query(&self) -> &str {
        self.inner
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("")
    }

    /// Retrieves all the header values for the given header field name
    pub fn header(&self, header_name: &str) -> Vec<&str> {
        self.inner
            .headers()
            .get_all(header_name)
            .iter()
            .map(|item| item.to_str().unwrap())
            .collect::<Vec<&str>>()
    }

    /// Checks whether the provided header field exists
    pub fn has_header(&self, header_name: &str) -> bool {
        self.inner.headers().contains_key(header_name)
    }

    /// Returns the request body or an error, if the body hasn't been read
    /// up to this moment.
    pub fn body(&self) -> Result<&Vec<u8>, Error> {
        self.body
            .as_ref()
            .ok_or_else(|| Error::new(ErrorKind::RequestBodyFailure))
    }

    /// Reads the body (if it hasn't been read already) and returns it
    pub(crate) async fn read_body(&mut self) -> &Vec<u8> {
        if self.body.is_none() {
            let raw_body = self.inner.body_mut();
            let mut buf = body::aggregate(raw_body)
                .await
                .map_err(|err| Error::new_with_context(ErrorKind::RequestBodyFailure, err))
                .unwrap();
            let bytes = buf.copy_to_bytes(buf.remaining()).to_vec();
            self.body = Some(bytes);
        }

        self.body.as_ref().unwrap()
    }

    pub(crate) fn formatted(&self) -> String {
        let mut formatted = format!(
            "\r\n{} {}\r\n",
            &self.inner.method(),
            &self
                .inner
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("")
        );

        for (key, value) in self.inner.headers() {
            formatted.push_str(&format!(
                "{}: {}\r\n",
                key,
                value.to_str().unwrap_or("<invalid>")
            ));
        }

        if let Some(body) = &self.body {
            if !body.is_empty() {
                formatted.push_str(&format!("{}\r\n", &String::from_utf8_lossy(body)));
            }
        }

        formatted
    }
}
