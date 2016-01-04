extern crate mockable;
use mockable::server;

fn main() {
    server::start(Some(1234));

    println!("server running at: {}", server::host());

    loop {}
}
