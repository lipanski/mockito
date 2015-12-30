#[cfg(feature = "use_hyper")]
extern crate hyper;
extern crate mockable;

#[cfg(feature = "use_hyper")]
use hyper::Client;
#[cfg(feature = "use_hyper")]
use hyper::header::Connection;

use mockable::server;
use mockable::url::Url;

use std::io::Read;

fn main() {
    server::start();

    call();
}

#[cfg(feature = "use_hyper")]
fn call() {
    let client = Client::new();
    let mut res = client.get(Url("http://www.example.com"))
        .header(Connection::close())
        .send()
        .unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("body: {}", body);
}

#[cfg(not(feature = "use_hyper"))]
fn call() {
    println!("not using hyper");
}
