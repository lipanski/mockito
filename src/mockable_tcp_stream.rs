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

#[cfg(test)]
#[cfg(not(feature = "mock_tcp_stream"))]
mod tests {
    use url::Url;
    use std::net::{ToSocketAddrs, SocketAddr, SocketAddrV4, Ipv4Addr};
    use std::str::FromStr;

    #[test]
    fn test_url_from_str_is_ok() {
        let url = Url("1.2.3.4:5678");

        assert!(url.to_socket_addrs().is_ok());
    }

    #[test]
    fn test_url_from_str_points_to_original_url() {
        let url = Url("1.2.3.4:5678");

        let expected_ip = Ipv4Addr::from_str("1.2.3.4").unwrap();
        let expected_port = 5678;
        let expected_url = SocketAddr::V4(SocketAddrV4::new(expected_ip, expected_port));

        assert_eq!(expected_url, url.to_socket_addrs().unwrap().last().unwrap());
    }
}

#[cfg(test)]
#[cfg(feature = "mock_tcp_stream")]
mod mock_tcp_stream_tests {
    use url::Url;
    use server::MockServer;
    use std::net::{ToSocketAddrs, SocketAddr, SocketAddrV4, Ipv4Addr};
    use std::str::FromStr;

    #[test]
    fn test_mocked_url_from_str_is_ok() {
        let url = Url("1.2.3.4:5678");

        assert!(url.to_socket_addrs().is_ok());
    }

    #[test]
    fn test_mocked_url_from_str_points_to_localhost() {
        let url = Url("1.2.3.4:5678");

        let expected_ip = Ipv4Addr::from_str("127.0.0.1").unwrap();
        let expected_port = MockServer::port() as u16;
        let expected_url = SocketAddr::V4(SocketAddrV4::new(expected_ip, expected_port));

        assert_eq!(expected_url, url.to_socket_addrs().unwrap().last().unwrap());
    }
}
