use std::io::Read;
use std::mem;
use std::str;
use std::convert::From;
use std::default::Default;
use std::net::TcpStream;
use std::fmt;

use httparse;

#[derive(Debug)]
pub struct Request {
    pub version: (u8, u8),
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
    pub fn is_head(&self) -> bool {
        self.method == "HEAD"
    }

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
        self.headers.iter().filter_map(|(field, value)| {
            if field == searched_field {
                Some(value.as_str())
            } else {
                None
            }
        }).collect()
    }

    fn record_last_header(&mut self) {
        if self.last_header_field.is_some() && self.last_header_value.is_some() {
            let last_header_field = mem::replace(&mut self.last_header_field, None).unwrap();
            let last_header_value = mem::replace(&mut self.last_header_value, None).unwrap();
            self.headers.push((last_header_field.to_lowercase(), last_header_value));
        }
    }

    fn content_length(&self) -> usize {
        use std::str::FromStr;

        self.find_header_values("content-length")
            .first()
            .and_then(|len| usize::from_str(*len).ok())
            .unwrap_or(0)
    }

    fn read_request_body(&mut self, stream: &mut TcpStream) {
        let expected_content_length = self.content_length();

        loop {
            if self.body.len() == expected_content_length {
                break;
            }

            let mut body_buf = [0; 1024];

            let body_read_len = stream.read(&mut body_buf).unwrap_or_else(|e| {
                self.error = Some(e.to_string());
                0
            });

            self.body.extend_from_slice(&body_buf[..body_read_len]);
        }
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
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
        let mut request = Self::default();

        let mut all_buf = Vec::new();

        loop {
            if request.is_parsed {
                break;
            }

            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Request::new(&mut headers);
            let mut buf = [0; 1024];

            let rlen = match stream.read(&mut buf) {
                Err(e) => Err(e.to_string()),
                Ok(0) => Err("Nothing to read.".into()),
                Ok(i) => Ok(i),
            }
            .map_err(|e| request.error = Some(e))
            .unwrap_or(0);

            if rlen == 0 {
                break;
            }

            all_buf.extend_from_slice(&buf[..rlen]);

            let _ = req
                .parse(&all_buf)
                .map_err(|e| {
                    request.error = Some(e.to_string());
                })
                .and_then(|status| match status {
                    httparse::Status::Complete(head_len) => {
                        if let Some(a @ 0...1) = req.version {
                            request.version = (1, a)
                        }
                        if let Some(a) = req.method {
                            request.method += a
                        }
                        if let Some(a) = req.path {
                            request.path += a
                        }
                        for h in req.headers {
                            request.last_header_field = Some(h.name.to_lowercase());
                            request.last_header_value =
                                Some(String::from_utf8_lossy(h.value).to_string());

                            request.record_last_header();
                        }

                        request.body.extend_from_slice(&all_buf[head_len..]);

                        let more_body_to_read = all_buf.len() < head_len + request.content_length();

                        if more_body_to_read {
                            request.read_request_body(stream);
                        }
                        request.is_parsed = true;

                        Ok(())
                    }
                    httparse::Status::Partial => Ok(()),
                });
        }

        request
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\r\n{} {}\r\n", &self.method, &self.path)?;

        for &(ref key, ref value) in &self.headers {
            writeln!(f, "{}: {}\r", key, value)?;
        }

        if self.body.is_empty() {
            write!(f, "")
        } else {
            writeln!(f, "{}\r", &String::from_utf8_lossy(&self.body))
        }
    }
}
