extern crate hyper;
extern crate intercepto;

use hyper::Client;
use hyper::header::Connection;

use intercepto::server;
use intercepto::url::Url;

use std::io::Read;

fn main() {
    server::start();

    let client = Client::new();
    let mut res = client.get(Url("http://www.example.com"))
        .header(Connection::close())
        .send()
        .unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("body: {}", body);
}
