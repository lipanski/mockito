extern crate hyper;
extern crate url;

use std::net::{TcpListener, TcpStream, SocketAddr, ToSocketAddrs};
use std::vec::IntoIter;
use std::io::{Write, Read};
use std::thread;
use std::str::FromStr;
use hyper::{Client, Url};
use hyper::header::Connection;
use hyper::client::IntoUrl;
use url::ParseError;

use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
static PROXY_PORT: AtomicUsize = ATOMIC_USIZE_INIT;

struct InterceptedUrl<'a>(&'a str);

impl<'a> InterceptedUrl<'a> {
    fn proxy_host() -> String {
        format!("http://127.0.0.1:{}", PROXY_PORT.load(Ordering::Acquire))
    }
}

impl<'a> ToSocketAddrs for InterceptedUrl<'a> {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        let mut res = Vec::new();

        let addr = SocketAddr::from_str(&Self::proxy_host());
        res.push(addr.unwrap());

        Ok(res.into_iter())
    }
}

impl<'a> IntoUrl for InterceptedUrl<'a> {
    fn into_url(self) -> Result<Url, ParseError> {
        Self::proxy_host().into_url()
    }
}

fn main() {
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

    let client = Client::new();
    let mut res = client.get(InterceptedUrl("http://www.example.com"))
        .header(Connection::close())
        .send()
        .unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("body: {}", body);
}

fn handle_client(mut stream: TcpStream) {
    let response = "HTTP/1.1 200 OK\n\nHello world";

    stream.write(response.as_bytes()).unwrap();
}
