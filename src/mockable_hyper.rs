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
