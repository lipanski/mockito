use std::time::Duration;

fn main() {
    let opts = mockito::ServerOpts {
        host: "0.0.0.0",
        port: 1234,
        ..Default::default()
    };
    let mut server = mockito::Server::new_with_opts(opts);

    let _m = server.mock("GET", "/").with_body("hello world").create();

    loop {
        std::thread::sleep(Duration::from_secs(1))
    }
}
