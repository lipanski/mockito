use std::io::Read;
use std::mem;
use std::str;
use std::convert::From;
use std::default::Default;
use std::net::TcpStream;
use std::fmt;

#[derive(Debug)]
pub struct Request {
    version: (u8, u8),
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    error: Option<String>,
    is_parsed: bool,
    last_header_field: Option<String>,
    last_header_value: Option<String>,
}

impl Request {
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    pub fn error(&self) -> Option<&String> {
        self.error.as_ref()
    }

    pub fn find_header_values(&self, searched_field: &str) -> Vec<&str> {
        self.headers.iter().filter(|&&(ref field, _)| field == searched_field)
            .map(|&(_, ref value)| value.as_str())
            .collect()
    }

    fn record_last_header(&mut self) {
        if self.last_header_field.is_some() && self.last_header_value.is_some() {
            let last_header_field = mem::replace(&mut self.last_header_field, None).unwrap();
            let last_header_value = mem::replace(&mut self.last_header_value, None).unwrap();
            self.headers.push((last_header_field.to_lowercase(), last_header_value));
        }
    }
}

impl Default for Request {
    fn default() -> Self {
        Request {
            version: (1, 1),
            method: String::new(),
            path: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
            error: None,
            is_parsed: false,
            last_header_field: None,
            last_header_value: None,
        }
    }
}

impl<'a> From<&'a mut TcpStream> for Request {
    fn from(stream: &mut TcpStream) -> Self {
        let mut request = Request::default();
        let mut buf = [0; 1024];

        let rlen = match stream.read(&mut buf) {
            Err(e) => Err(e.to_string()),
            Ok(0)  => Err("Nothing to read.".into()),
            Ok(i)  => Ok(i)
        }.map_err(|e| request.error = Some(e)).unwrap_or(0);
        if rlen == 0 {
            return request;
        }

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        let _ = req.parse(&buf).map_err(|e|{
            request.error = Some(e.to_string());
        }).and_then(|p| {
            if let Some(a @ 0 ... 1) = req.version { request.version = (1,a) }
            if let Some(a)           = req.method  { request.method += a }
            if let Some(a)           = req.path    { request.path += a }
            for h in req.headers {
                request.last_header_field = Some(h.name.to_lowercase());
                request.last_header_value = Some(String::from_utf8_lossy(h.value).into());
                request.record_last_header();
            }
            if let httparse::Status::Complete(plen) = p {
                request.is_parsed = true;
                request.body.extend_from_slice(&buf[plen..rlen]);
            }
            Ok(())
        });
        request
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\r\n{} {}\r\n", &self.method, &self.path)?;

        for &(ref key, ref value) in &self.headers {
            write!(f, "{}: {}\r\n", key, value)?;
        }

        if !self.body.is_empty() {
            write!(f, "{}\r\n", &String::from_utf8_lossy(&self.body))
        } else {
            write!(f, "")
        }
    }
}
