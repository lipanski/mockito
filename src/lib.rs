#[cfg(feature = "mock_hyper")]
extern crate hyper;
#[cfg(feature = "mock_hyper")]
extern crate url;

pub mod server;
pub mod intercepted_url;
#[cfg(feature = "mock_hyper")]
pub mod intercept_hyper;
#[cfg(feature = "mock_tcp_listener")]
pub mod tcp_listener;

pub type InterceptedUrl<'a> = intercepted_url::InterceptedUrl<'a>;

#[cfg(test)]
mod tests {
    use hyper::Client;
    use hyper::header::Connection;
    use server;
    use intercepted_url::InterceptedUrl;
    use std::io::Read;

    #[test]
    #[cfg(feature = "mock_hyper")]
    fn test_proxying() {
        server::init();

        let client = Client::new();
        let mut res = client.get(InterceptedUrl("http://www.example.com"))
            .header(Connection::close())
            .send()
            .unwrap();

        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();

        assert_eq!(body, "Hello world");
    }
}
