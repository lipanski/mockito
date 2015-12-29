use server;
use std::sync::atomic::Ordering;

pub struct Url<'a>(pub &'a str);

impl<'a> Url<'a> {
    pub fn proxy_host() -> String {
        format!("127.0.0.1:{}", Self::proxy_port())
    }

    pub fn proxy_host_with_protocol() -> String {
        format!("http://127.0.0.1:{}", Self::proxy_port())
    }

    pub fn proxy_port() -> usize {
        server::PORT.load(Ordering::SeqCst)
    }
}
