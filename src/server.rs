use std::net::{TcpStream, TcpListener};
use std::io::Write;
use std::thread;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering, ATOMIC_USIZE_INIT, ATOMIC_BOOL_INIT};

pub static PORT: AtomicUsize = ATOMIC_USIZE_INIT;
pub static SERVER_THREAD_SPAWNED: AtomicBool = ATOMIC_BOOL_INIT;

pub fn start() {
    if is_server_thread_spawned() { return }

    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port     = listener.local_addr().unwrap().port() as usize;

        PORT.store(port, Ordering::SeqCst);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_client(stream),
                Err(e)     => println!("Error: {}", e)
            }
        }

        drop(listener);
    });

    SERVER_THREAD_SPAWNED.store(true, Ordering::SeqCst);

    while !is_running() {}
}

pub fn is_server_thread_spawned() -> bool {
    SERVER_THREAD_SPAWNED.load(Ordering::SeqCst)
}

pub fn is_running() -> bool {
    port() != 0
}

pub fn port() -> usize {
    PORT.load(Ordering::SeqCst)
}

pub fn host() -> String {
    format!("127.0.0.1:{}", port())
}

pub fn host_with_protocol() -> String {
    format!("http://127.0.0.1:{}", port())
}

fn handle_client(mut stream: TcpStream) {
    let response = "HTTP/1.1 200 OK\n\nHello world";

    stream.write(response.as_bytes()).unwrap();
}

#[cfg(test)]
mod tests {
    use server;

    #[test]
    fn test_start_server() {
        server::start();

        assert!(server::is_server_thread_spawned());
        assert!(server::is_running());
        assert!(server::port() != 0);
    }
}
