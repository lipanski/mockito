use mockito;

use std::time::Duration;

fn main() {
    let mut s = mockito::Server::new();

    s.mock("GET", "/").with_body("hello world");

    loop {
        std::thread::sleep(Duration::from_secs(1))
    }
}
