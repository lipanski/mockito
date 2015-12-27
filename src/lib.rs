#[cfg(feature = "mock_hyper")]
extern crate hyper;
#[cfg(feature = "mock_hyper")]
extern crate url as servo_url;

pub mod server;
pub mod url;
#[cfg(feature = "mock_hyper")]
pub mod mock_hyper;
#[cfg(feature = "mock_tcp_listener")]
pub mod mock_tcp_listener;

pub type Url<'a> = url::Url<'a>;

#[cfg(test)]
mod tests {
    use hyper::Client;
    use hyper::header::Connection;
    use server;
    use url::Url;
    use std::io::Read;

    #[test]
    #[cfg(feature = "mock_hyper")]
    fn test_proxying() {
        server::init();

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
