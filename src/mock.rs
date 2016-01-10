use server::Request;
use client;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

pub struct Mock {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    response: Option<String>
}

impl Mock {
    pub fn new(method: &str, path: &str) -> Mock {
        Mock {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            response: None
        }
    }

    pub fn from(request: &Request) -> Option<Mock> {
        match (request.headers.get("x-mock-method"), request.headers.get("x-mock-path")) {
            (Some(ref mock_method), Some(ref mock_path)) => {
                let mut headers = HashMap::new();

                for (field, value) in &request.headers {
                    if field.starts_with("x-mock-") && field != "x-mock-method" && field != "x-mock-path" {
                        headers.insert(field.replace("x-mock-", ""), value.clone());
                    }
                }

                let mock = Mock {
                    method: mock_method.to_string(),
                    path: mock_path.to_string(),
                    headers: headers,
                    response: Some(request.body.to_string())
                };

                Some(mock)
            },
            _ => None
        }
    }

    pub fn header(mut self, field: &str, value: &str) -> Self {
        self.headers.insert(field.to_string(), value.to_string());
        self
    }

    pub fn respond_with(mut self, response: &str) -> Self {
        self.response = Some(response.to_string());
        self.register();
        self
    }

    pub fn respond_with_file(mut self, path: &str) -> Self {
        let mut file = File::open(path).unwrap();
        let mut response = String::new();
        file.read_to_string(&mut response).unwrap();
        self.response = Some(response);
        self.register();
        self
    }

    pub fn register(&self) {
        client::new(self);
    }

    pub fn response(&self) -> String {
        match self.response.as_ref() {
            Some(response) => response.clone(),
            None => "HTTP/1.1 OK 200\n\n".to_string()
        }
    }

    pub fn matches(&self, request: &Request) -> bool {
        self.method_matches(request)
            && self.path_matches(request)
            && self.headers_match(request)
    }

    fn method_matches(&self, request: &Request) -> bool {
        request.method == self.method
    }

    fn path_matches(&self, request: &Request) -> bool {
        request.path == self.path
    }

    fn headers_match(&self, request: &Request) -> bool {
        for (field, value) in self.headers.iter() {
            match request.headers.get(field) {
                Some(request_value) if request_value == value => continue,
                _ => return false
            }
        }

        true
    }
}
