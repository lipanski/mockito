use std::thread::{self, sleep};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;
use hyper::server::{Handler, Server, Request, Response};
use hyper::header::ContentLength;
use hyper::method::{Method};
use {Mock, SERVER_ADDRESS};

static SERVER_STARTED: AtomicBool = ATOMIC_BOOL_INIT;

#[derive(Debug)]
enum CreateMockError {
    MethodMissing,
    PathMissing,
    ContentLengthMissing,
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
    fn handle_create_mock(&self, request: Request, response: Response) {
        match Self::mock_from(request) {
            Ok(mock) => {
                self.mocks.lock().unwrap().push(mock);
                response.send(b"HTTP/1.1 201 OK\n\n").unwrap();
            },
            Err(e) => {
                // TODO: implement Display for CreateMockError
                response.send(b"HTTP/1.1 422 Unprocessable Entity\n\n").unwrap();
            }
        }
    }

    #[allow(unused_variables)]
    fn handle_delete_mocks(&self, request: Request, response: Response) {
        self.mocks.lock().unwrap().clear();

        response.send(b"HTTP/1.1 200 OK\n\n").unwrap();
    }

    fn handle_default(&self, request: Request, response: Response) {
        let mocks = self.mocks.lock().unwrap();

        match mocks.iter().rev().find(|mock| mock.matches(&request)).and_then(|mock| mock.response()) {
            Some(value) => { response.send(value.as_bytes()).unwrap(); },
            None => { response.send(b"HTTP/1.1 501 Not Implemented\n\n").unwrap(); },
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
                mock.header(&field.replace("x-mock-", ""), &header.value_string());
            }
        }

        let content_length: ContentLength = *try!(request.headers.get().ok_or(CreateMockError::ContentLengthMissing));

        let mut body = String::new();
        request.take(content_length.0).read_to_string(&mut body).unwrap();

        mock.set_response(body);

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
    if SERVER_STARTED.load(Ordering::SeqCst) { return }

    SERVER_STARTED.store(true, Ordering::SeqCst);

    start()
}

fn start() {
    let mocks: Arc<Mutex<Vec<Mock>>> = Arc::new(Mutex::new(vec!()));

    thread::spawn(move || {
        Server::http(SERVER_ADDRESS).unwrap().handle(RequestHandler::new(mocks)).unwrap();
    });

    while !is_listening() {}
}

fn is_listening() -> bool {
    TcpStream::connect(SERVER_ADDRESS).is_ok()
}
