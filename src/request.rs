use crate::{Error, ErrorKind};
use hyper::body;
use hyper::body::Buf;
use hyper::Body as HyperBody;
use hyper::Request as HyperRequest;

#[derive(Debug)]
pub(crate) struct Request {
    inner: HyperRequest<HyperBody>,
    body: Option<Vec<u8>>,
}

impl Request {
    pub fn new(request: HyperRequest<HyperBody>) -> Self {
        Request {
            inner: request,
            body: None,
        }
    }

    pub fn method(&self) -> &str {
        self.inner.method().as_ref()
    }

    pub fn path_and_query(&self) -> &str {
        self.inner
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("")
    }

    pub fn header(&self, field: &str) -> Vec<&str> {
        self.inner
            .headers()
            .get_all(field)
            .iter()
            .map(|item| item.to_str().unwrap())
            .collect::<Vec<&str>>()
    }

    pub fn has_header(&self, header_name: &str) -> bool {
        self.inner.headers().contains_key(header_name)
    }

    pub async fn read_body(&mut self) -> &Vec<u8> {
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

    #[allow(clippy::wrong_self_convention)]
    pub(crate) async fn to_string(&mut self) -> String {
        let mut formatted = format!(
            "\r\n{} {}\r\n",
            &self.inner.method(),
            &self.inner.uri().path()
        );

        for (key, value) in self.inner.headers() {
            formatted.push_str(&format!(
                "{}: {}\r\n",
                key,
                value.to_str().unwrap_or("<invalid>")
            ));
        }

        let body = self.read_body().await;

        if !body.is_empty() {
            formatted.push_str(&format!("{}\r\n", &String::from_utf8_lossy(body)));
        }

        formatted
    }
}
