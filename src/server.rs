use MockBuilder;

use std::net::{TcpStream, TcpListener};
use std::io::{Write, Read, BufReader, BufRead};
use std::thread;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering, ATOMIC_USIZE_INIT, ATOMIC_BOOL_INIT};
use std::sync::{Arc, Mutex};

pub static PORT: AtomicUsize = ATOMIC_USIZE_INIT;
pub static SERVER_THREAD_SPAWNED: AtomicBool = ATOMIC_BOOL_INIT;
pub static REQUEST_SERVER_STOP: AtomicBool = ATOMIC_BOOL_INIT;
pub static STOP_REQUEST: &'static [u8] = b"STOP";

pub fn instance() {
    if is_listening() { return };

    start()
}

fn start() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    set_port(port);

    thread::spawn(move || {
        let mut mocks = Vec::new();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_request(stream, &mut mocks),
                Err(e)     => println!("Error: {}", e)
            }
        }

        drop(listener);
    });
}

pub fn new(mock: String) {
    let mut stream = TcpStream::connect(&*host()).unwrap();

    stream.write_all(b"POST /mockito/mocks HTTP/1.1\n\n");
    stream.write_all(mock.as_bytes());
}

pub fn reset() {
    let mut stream = TcpStream::connect(&*host()).unwrap();

    stream.write_all(b"DELETE /mockito/mocks HTTP/1.1\n\n");
}

pub fn is_listening() -> bool {
    TcpStream::connect(&*host()).is_ok()
}

pub fn port() -> u16 {
    PORT.load(Ordering::SeqCst) as u16
}

fn set_port(port: u16) {
    PORT.store(port as usize, Ordering::SeqCst);
}

pub fn host() -> String {
    format!("127.0.0.1:{}", port())
}

pub fn host_with_protocol() -> String {
    format!("http://127.0.0.1:{}", port())
}

fn handle_request(stream: TcpStream, mocks: &mut Vec<String>) {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
}

fn handle_command() {}

fn handle_mock() {}

pub struct MockServer {
    // listener_mutex: Arc<Mutex<TcpListener>>,
    port: u16
}

impl MockServer {
    pub fn new(mocks: Vec<MockBuilder>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        Self::set_port(port);

        // let listener_mutex = Arc::new(Mutex::new(listener));
        // let cloned_listener = listener_mutex.clone();

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let mut buf = [0; 4];
                        stream.read(&mut buf);

                        if buf == STOP_REQUEST { break }

                        Self::handle_client(stream, &mocks)
                    },
                    Err(e) => println!("Error: {}", e)
                }
            }
        });

        MockServer {
            // listener_mutex: listener_mutex,
            port: port
        }
    }

    pub fn is_listening(&self) -> bool {
        TcpStream::connect(&*Self::host()).is_ok()
    }

    pub fn stop(&mut self) {
        let _ = TcpStream::connect(&*Self::host()).and_then(|mut stream| stream.write_all(STOP_REQUEST));

        while self.is_listening() {}

        Self::set_port(0);
    }

    pub fn port() -> u16 {
        PORT.load(Ordering::SeqCst) as u16
    }

    pub fn host() -> String {
        format!("127.0.0.1:{}", Self::port())
    }

    pub fn host_with_protocol() -> String {
        format!("http://127.0.0.1:{}", Self::port())
    }

    fn set_port(port: u16) {
        PORT.store(port as usize, Ordering::SeqCst);
    }

    fn handle_client(mut stream: TcpStream, mocks: &Vec<MockBuilder>) {
        let response = "HTTP/1.1 200 OK\n\nHello world";

        stream.write(response.as_bytes()).unwrap();
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop()
    }
}

#[cfg(test)]
mod mock_server_tests {
    use server::MockServer;

    #[test]
    fn test_new_server_is_listening() {
        let server = MockServer::new(vec!());

        assert!(server.is_listening());
    }

    #[test]
    fn test_server_has_port() {
        let server = MockServer::new(vec!());

        assert!(server.port != 0);
    }

    #[test]
    fn test_server_stop() {
        let mut server = MockServer::new(vec!());
        assert!(server.is_listening());

        server.stop();
        assert!(!server.is_listening());
    }

    #[test]
    fn test_dropping_server() {
        assert!(MockServer::port() == 0);

        {
            let server = MockServer::new(vec!());
            assert!(MockServer::port() != 0);
        }

        assert!(MockServer::port() == 0);
    }
}
