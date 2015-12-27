use server;
use std::sync::atomic::Ordering;

pub struct Url<'a>(pub &'a str);

impl<'a> Url<'a> {
    pub fn proxy_host() -> String {
        format!("http://127.0.0.1:{}", server::PORT.load(Ordering::Acquire))
    }
}
