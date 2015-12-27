use server;
use std::sync::atomic::Ordering;

pub struct InterceptedUrl<'a>(pub &'a str);

impl<'a> InterceptedUrl<'a> {
    pub fn proxy_host() -> String {
        format!("http://127.0.0.1:{}", server::PROXY_PORT.load(Ordering::Acquire))
    }
}
