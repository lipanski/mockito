use hyper::client::IntoUrl;
use hyper::Url;

use intercepted_url::InterceptedUrl;

use url::ParseError;

impl<'a> IntoUrl for InterceptedUrl<'a> {
    #[cfg(not(test))]
    fn into_url(self) -> Result<Url, ParseError> {
        self.0.into_url()
    }

    #[cfg(test)]
    fn into_url(self) -> Result<Url, ParseError> {
        Self::proxy_host().into_url()
    }
}
