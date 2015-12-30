#[cfg(feature = "use_hyper")]
extern crate hyper;
#[cfg(feature = "use_hyper")]
extern crate url as servo_url;

pub mod server;
pub mod url;
#[cfg(feature = "use_hyper")]
pub mod mock_hyper;
pub mod mock_tcp_stream;

pub type Url<'a> = url::Url<'a>;

pub fn start() {
    server::start();
}

#[cfg(test)]
#[cfg(feature = "mock_hyper")]
mod mock_hyper_tests {
    use hyper::Client;
    use hyper::header::Connection;
    use server;
    use url::Url;
    use std::io::Read;

    #[test]
    fn test_proxying() {
        server::start();

        let client = Client::new();
        let mut res = client.get(Url("http://www.example.com"))
            .header(Connection::close())
            .send()
            .unwrap();

        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();

        assert_eq!(body, "Hello world");
    }
}
