use crate::{Matcher, Mock, Server};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::sync::LockResult;
use std::sync::{Mutex, MutexGuard};
lazy_static! {
    // Legacy mode.
    // A global lock that ensure all Mockito tests are run on a single thread.
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());

    // Legacy mode.
    static ref DEFAULT_SERVER: Mutex<Server> = Mutex::new(Server::new_with_port(0));
}
thread_local!(
    // Legacy mode.
    // A thread-local reference to the global lock. This is acquired within `mock()`.
    pub(crate) static LOCAL_TEST_MUTEX: RefCell<LockResult<MutexGuard<'static, ()>>> =
        RefCell::new(TEST_MUTEX.lock());
);

///
/// **DEPRECATED:** This method is part of the legacy interface an will be removed
/// in future versions. You should replace it with `Server::mock`:
///
/// ```
/// let mut s = mockito::Server::new();
/// let _m1 = s.mock("GET", "/");
/// ```
///
/// Initializes a mock with the given HTTP `method` and `path`.
///
/// The mock is registered to the server only after the `create()` method has been called.
///
#[deprecated(since = "0.32.0", note = "Use `Server::mock` instead")]
pub fn mock<P: Into<Matcher>>(method: &str, path: P) -> Mock {
    // Legacy mode.
    // Ensures Mockito tests are run sequentially.
    LOCAL_TEST_MUTEX.with(|_| {});

    let mut server = DEFAULT_SERVER.lock().unwrap();

    server.mock(method, path)
}

///
/// **DEPRECATED:** This method is part of the legacy interface an will be removed
/// in future versions. You should replace it with `Server::host_with_port`:
///
/// ```
/// let mut s = mockito::Server::new();
/// let server_address = s.host_with_port();
/// ```
///
/// The host and port of the local server.
/// Can be used with `std::net::TcpStream`.
///
#[deprecated(since = "0.32.0", note = "Use `Server::host_with_port` instead")]
pub fn server_address() -> String {
    let server = DEFAULT_SERVER.lock().unwrap();
    server.host_with_port()
}

///
/// **DEPRECATED:** This method is part of the legacy interface an will be removed
/// in future versions. You should replace it with `Server::url`:
///
/// ```
/// let mut s = mockito::Server::new();
/// let server_url = s.url();
/// ```
///
/// The local `http://...` URL of the server.
///
#[deprecated(since = "0.32.0", note = "Use `Server::url` instead")]
pub fn server_url() -> String {
    let server = DEFAULT_SERVER.lock().unwrap();
    server.url()
}

///
/// **DEPRECATED:** This method is part of the legacy interface an will be removed
/// in future versions. You should replace it with `Server::reset`:
///
/// ```
/// let mut s = mockito::Server::new();
/// s.reset();
/// ```
///
/// Removes all the mocks stored on the server.
///
#[deprecated(since = "0.32.0", note = "Use `Server::reset` instead")]
pub fn reset() {
    let mut server = DEFAULT_SERVER.lock().unwrap();
    server.reset();
}
