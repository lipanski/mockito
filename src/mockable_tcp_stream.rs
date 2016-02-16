use Url;
#[cfg(feature = "mock_tcp_stream")]
use server;

use std::net::{SocketAddr, ToSocketAddrs};
use std::vec::IntoIter;
use std::io::Result;

impl<'a> ToSocketAddrs for Url<'a> {
    type Iter = IntoIter<SocketAddr>;

    #[cfg(not(feature = "mock_tcp_stream"))]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        self.0.to_socket_addrs()
    }

    #[cfg(feature = "mock_tcp_stream")]
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        let host = server::host();

        host.to_socket_addrs()
    }
}
