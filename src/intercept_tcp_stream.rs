use intercepted_url::InterceptedUrl;

use std::net::{SocketAddr, ToSocketAddrs};
use std::vec::IntoIter;
use std::io::Result;
#[cfg(test)]
use std::str::FromStr;

impl<'a> ToSocketAddrs for InterceptedUrl<'a> {
    type Iter = IntoIter<SocketAddr>;

    #[cfg(not(test))]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        self.0.to_socket_addrs()
    }

    #[cfg(test)]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        let mut res = Vec::new();

        let addr = SocketAddr::from_str(&Self::proxy_host());
        res.push(addr.unwrap());

        Ok(res.into_iter())
    }
}
