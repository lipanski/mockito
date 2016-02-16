#[cfg(feature = "use_hyper")]
extern crate hyper;
#[cfg(feature = "use_hyper")]
extern crate url as servo_url;

pub mod server;
pub mod client;
pub mod mock;
pub mod url;
#[cfg(feature = "use_hyper")]
pub mod mockable_hyper;
pub mod mockable_tcp_stream;

pub type Url<'a> = url::Url<'a>;
pub type Mock = mock::Mock;

pub fn mock(method: &str, path: &str) -> Mock {
    server::listen();

    Mock::new(method, path)
}

pub fn reset() {
    client::reset();
}
