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
        Self::proxy_host().into_url()
    }
}
