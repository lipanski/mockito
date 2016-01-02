use std::collections::HashMap;

pub struct MockBuilder {
    pub request_line: String,
    pub headers: HashMap<String, String>,
    pub response: Option<String>
}

impl MockBuilder {
    pub fn new(request_line: &str) -> Self {
        MockBuilder {
            request_line: request_line.to_string(),
            headers: HashMap::new(),
            response: None
        }
    }

    pub fn header(&mut self, field: &str, value: &str) -> &mut Self {
        self.headers.insert(field.to_string(), value.to_string());
        self
    }

    pub fn with(&mut self) -> &mut Self {
        self
    }
}
