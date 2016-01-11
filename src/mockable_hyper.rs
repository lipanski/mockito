use hyper::client::IntoUrl;
use hyper::Url as HyperUrl;
use servo_url::ParseError;
use servo_url::Url as ServoUrl;

use Url;
#[cfg(feature = "mock_hyper")]
use server;

impl<'a> IntoUrl for Url<'a> {
    #[cfg(not(feature = "mock_hyper"))]
    fn into_url(self) -> Result<HyperUrl, ParseError> {
        self.0.into_url()
    }

    #[cfg(feature = "mock_hyper")]
    fn into_url(self) -> Result<HyperUrl, ParseError> {
        let parsed_url = try!(ServoUrl::parse(self.0));

        let mut url = server::host_with_protocol();
        url = url + &parsed_url.serialize_path().unwrap_or(String::new());
        url = url + &parsed_url.query.map(|q| "?".to_string() + &q).unwrap_or(String::new());

        url.into_url()
    }
}

#[cfg(test)]
#[cfg(not(feature = "mock_hyper"))]
mod tests {
    use hyper::client::IntoUrl;
    use hyper::Url as HyperUrl;
    use url::Url;

    #[test]
    fn test_url_from_str_is_ok() {
        let url = Url("https://www.exmaple.com");

        assert!(url.into_url().is_ok());
    }

    #[test]
    fn test_url_from_str_points_to_original_url() {
        let url = Url("https://www.example.com");
        let expected_url = HyperUrl::parse("https://www.example.com").unwrap();

        assert_eq!(expected_url, url.into_url().ok().unwrap());
    }
}

#[cfg(test)]
#[cfg(feature = "mock_hyper")]
mod mock_hyper_tests {
    use hyper::client::IntoUrl;
    use hyper::Url as HyperUrl;
    use url::Url;
    use server::MockServer;

    #[test]
    fn test_mocked_url_from_str_is_ok() {
        let url = Url("https://www.example.com");

        assert!(url.into_url().is_ok());
    }

    #[test]
    fn test_mocked_url_from_str_points_to_localhost() {
        let url = Url("https://www.example.com");
        let expected_url = HyperUrl::parse(&MockServer::host_with_protocol()).unwrap();

        assert_eq!(expected_url, url.into_url().ok().unwrap());
    }
}
