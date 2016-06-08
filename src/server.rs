use std::thread::{self};
use std::sync::{Arc, Mutex};
use std::io::Read;
use std::net::TcpStream;
use std::mem;
use hyper::server::{Handler, Server, Request, Response};
use hyper::header::ContentLength;
use hyper::method::{Method};
use hyper::status::StatusCode;
use rustc_serialize::json;
use {Mock, MockResponse, SERVER_ADDRESS};

#[derive(Debug)]
enum CreateMockError {
    MethodMissing,
    PathMissing,
    ContentLengthMissing,
    InvalidMockResponse,
}

struct RequestHandler {
    mocks: Arc<Mutex<Vec<Mock>>>,
}

impl RequestHandler {
    pub fn new(mocks: Arc<Mutex<Vec<Mock>>>) -> Self {
        RequestHandler {
            mocks: mocks,
        }
    }

    #[allow(unused_variables)]
    fn handle_list_mocks(&self, request: Request, response: Response) {
        let mocks = self.mocks.lock().unwrap();
        // TODO: implement Display for Mock
    }

    #[allow(unused_variables)]
    fn handle_create_mock(&self, request: Request, mut response: Response) {
        match Self::mock_from(request) {
            Ok(mock) => {
                self.mocks.lock().unwrap().push(mock);
            },
            Err(e) => {
                // TODO: implement Display for CreateMockError
                mem::replace(response.status_mut(), StatusCode::UnprocessableEntity);
            }
        }
    }

    #[allow(unused_variables)]
    fn handle_delete_mocks(&self, request: Request, response: Response) {
        self.mocks.lock().unwrap().clear();
    }

    fn handle_default(&self, request: Request, mut response: Response) {
        let mocks = self.mocks.lock().unwrap();

        match mocks.iter().rev().find(|mock| mock.matches(&request)) {
            Some(mock) => {
                // Set the response status code
                // TODO: StatusCode::Unregistered labels everything as `<unknown status code>`
                mem::replace(response.status_mut(), StatusCode::Unregistered(mock.response.status as u16));

                // Set the response headers
                for (field, value) in &mock.response.headers {
                    response.headers_mut().set_raw(field.to_owned(), vec!(value.as_bytes().to_vec()));
                }

                // Set the response body
                response.send(mock.response.body.as_bytes()).unwrap();
            },
            None => { mem::replace(response.status_mut(), StatusCode::NotImplemented); },
        };
    }

    fn mock_from(request: Request) -> Result<Mock, CreateMockError> {
        let method: String = try!(
            request.headers.iter()
                .find(|header| { header.name().to_lowercase() == "x-mock-method" })
                .ok_or(CreateMockError::MethodMissing)
            ).value_string();

        let path: String = try!(
            request.headers.iter()
                .find(|header| { header.name().to_lowercase() == "x-mock-path" })
                .ok_or(CreateMockError::PathMissing)
            ).value_string();

        let mut mock = Mock::new(&method, &path);

        for header in request.headers.iter() {
            let field = header.name().to_lowercase();
            if field.starts_with("x-mock-") && field != "x-mock-method" && field != "x-mock-path" {
                mock.match_header(&field.replace("x-mock-", ""), &header.value_string());
            }
        }

        let content_length: ContentLength = *try!(request.headers.get().ok_or(CreateMockError::ContentLengthMissing));

        let mut body = String::new();
        request.take(content_length.0).read_to_string(&mut body).unwrap();

        let response: MockResponse = try!(json::decode(&body).map_err(|_| CreateMockError::InvalidMockResponse));
        mock.response = response;

        Ok(mock)
    }
}

impl Handler for RequestHandler {
    fn handle(&self, request: Request, response: Response) {
        match (&request.method, &*request.uri.to_string()) {
            (&Method::Get, "/mocks") => self.handle_list_mocks(request, response),
            (&Method::Post, "/mocks") => self.handle_create_mock(request, response),
            (&Method::Delete, "/mocks") => self.handle_delete_mocks(request, response),
            _ => self.handle_default(request, response),
        };
    }
}

pub fn try_start() {
    if is_listening() { return }

    start()
}

fn start() {
    let mocks: Arc<Mutex<Vec<Mock>>> = Arc::new(Mutex::new(vec!()));

    thread::spawn(move || {
        match Server::http(SERVER_ADDRESS) {
            Ok(server) => { server.handle(RequestHandler::new(mocks)).unwrap(); },
            Err(_) => {},
        };
    });

    while !is_listening() {}
}

fn is_listening() -> bool {
    TcpStream::connect(SERVER_ADDRESS).is_ok()
}
