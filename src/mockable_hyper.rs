use hyper::client::IntoUrl;
use hyper::Url as HyperUrl;
use servo_url::ParseError;

use Url;

impl<'a> IntoUrl for Url<'a> {
    #[cfg(not(feature = "mock_hyper"))]
    fn into_url(self) -> Result<HyperUrl, ParseError> {
        self.0.into_url()
    }

    #[cfg(feature = "mock_hyper")]
    fn into_url(self) -> Result<HyperUrl, ParseError> {
        Self::proxy_host_with_protocol().into_url()
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

    #[test]
    fn test_mocked_url_from_str_is_ok() {
        let url = Url("https://www.example.com");

        assert!(url.into_url().is_ok());
    }

    #[test]
    fn test_mocked_url_from_str_points_to_localhost() {
        let url = Url("https://www.example.com");
        let expected_url = HyperUrl::parse("http://127.0.0.1:0").unwrap();

        assert_eq!(expected_url, url.into_url().ok().unwrap());
    }
}
