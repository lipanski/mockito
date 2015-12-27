use hyper::client::IntoUrl;
use hyper::Url;

use intercepted_url::InterceptedUrl;

use url::ParseError;

impl<'a> IntoUrl for InterceptedUrl<'a> {
    #[cfg(not(feature = "mock_hyper"))]
    fn into_url(self) -> Result<Url, ParseError> {
        self.0.into_url()
    }

    #[cfg(feature = "mock_hyper")]
    fn into_url(self) -> Result<Url, ParseError> {
        Self::proxy_host().into_url()
    }
}
