use std::io::Read;
use std::mem;
use std::str;
use std::convert::From;
use std::default::Default;
use std::net::TcpStream;
use std::fmt;
use http_muncher::{Parser, ParserHandler};

#[derive(Debug)]
pub struct Request {
    version: (u16, u16),
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

    fn is_parsed(&self) -> bool {
        self.is_parsed
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
        let mut parser = Parser::request();
        let mut first_read = true;

        loop {
            if request.is_parsed() { break; }

            let mut buffer = [0; 1024];
            let read_length = stream.read(&mut buffer).unwrap_or(0);

            if read_length == 0 {
                if first_read { request.error = Some("Nothing to read.".to_string()); }
                break;
            }

            first_read = false;

            let parse_length = parser.parse(&mut request, (&buffer).chunks(read_length).nth(0).unwrap());
            if parse_length == 0 || parser.has_error() { break; }
        }

        if parser.has_error() {
            request.error = Some(parser.error().to_string());
        } else {
            request.version = parser.http_version();
            request.method = parser.http_method().to_string();
        }

        request
    }
}

impl ParserHandler for Request {
    fn on_message_begin(&mut self, parser: &mut Parser) -> bool {
        !parser.has_error()
    }

    fn on_url(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        self.path.push_str(str::from_utf8(value).unwrap());

        !parser.has_error()
    }

    fn on_header_field(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        self.record_last_header();

        if self.last_header_field.is_none() {
            self.last_header_field = Some(String::new());
        }

        (*self.last_header_field.as_mut().unwrap()).push_str(str::from_utf8(value).unwrap());

        !parser.has_error()
    }

    fn on_header_value(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        if self.last_header_value.is_none() {
            self.last_header_value = Some(String::new());
        }

        (*self.last_header_value.as_mut().unwrap()).push_str(str::from_utf8(value).unwrap());

        !parser.has_error()
    }

    fn on_headers_complete(&mut self, parser: &mut Parser) -> bool {
        self.record_last_header();

        !parser.has_error()
    }

    fn on_body(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        self.body.extend(value);

        !parser.has_error()
    }

    fn on_message_complete(&mut self, parser: &mut Parser) -> bool {
        self.is_parsed = true;

        !parser.has_error()
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatted = String::new();

        formatted.push_str("\r\n");
        formatted.push_str(&self.method);
        formatted.push_str(" ");
        formatted.push_str(&self.path);
        formatted.push_str("\r\n");

        for &(ref key, ref value) in &self.headers {
            formatted.push_str(key);
            formatted.push_str(": ");
            formatted.push_str(value);
            formatted.push_str("\r\n");
        }

        if self.body.len() > 0 {
            formatted.push_str(&String::from_utf8_lossy(&self.body));
            formatted.push_str("\r\n");
        }

        write!(f, "{}", formatted)
    }
}
