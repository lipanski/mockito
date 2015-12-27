extern crate hyper;
extern crate proxy;

use hyper::Client;
use hyper::header::Connection;

use proxy::server;
use proxy::intercepted_url::InterceptedUrl;

use std::io::Read;

fn main() {
    server::init();

    let client = Client::new();
    let mut res = client.get(InterceptedUrl("http://www.example.com"))
        .header(Connection::close())
        .send()
        .unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("body: {}", body);
}
