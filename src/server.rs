use std::net::{TcpStream, TcpListener};
use std::io::Write;
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

pub static PROXY_PORT: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn init() {
    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port() as usize;

        PROXY_PORT.fetch_add(port, Ordering::Release);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_client(stream),
                Err(e)     => println!("Error: {}", e)
            }
        }

        drop(listener);
    });

    while PROXY_PORT.load(Ordering::Acquire) == 0 {}
}

fn handle_client(mut stream: TcpStream) {
    let response = "HTTP/1.1 200 OK\n\nHello world";

    stream.write(response.as_bytes()).unwrap();
}
