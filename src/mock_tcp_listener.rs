use Url;

use std::net::{SocketAddr, ToSocketAddrs};
use std::vec::IntoIter;
use std::io::Result;
#[cfg(feature = "mock_tcp_listener")]
use std::str::FromStr;

impl<'a> ToSocketAddrs for Url<'a> {
    type Iter = IntoIter<SocketAddr>;

    #[cfg(not(feature = "mock_tcp_listener"))]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        self.0.to_socket_addrs()
    }

    #[cfg(feature = "mock_tcp_listener")]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        let mut res = Vec::new();

        let addr = SocketAddr::from_str(&Self::proxy_host());
        res.push(addr.unwrap());

        Ok(res.into_iter())
    }
}
