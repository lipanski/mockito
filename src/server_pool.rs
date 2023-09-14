use crate::Server;
use crate::{Error, ErrorKind};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::Mutex;
use tokio::sync::{Semaphore, SemaphorePermit};

// macOS has small default ulimits. Sync it with test_server_pool()
const DEFAULT_POOL_SIZE: usize = if cfg!(target_os = "macos") { 20 } else { 50 };
pub(crate) static SERVER_POOL: ServerPool = ServerPool::new(DEFAULT_POOL_SIZE);

///
/// A handle around a pooled `Server` object which dereferences to `Server`.
///
pub struct ServerGuard {
    server: Option<Server>,
    _permit: SemaphorePermit<'static>,
}

impl ServerGuard {
    fn new(server: Server, _permit: SemaphorePermit<'static>) -> ServerGuard {
        ServerGuard {
            server: Some(server),
            _permit,
        }
    }
}

impl Deref for ServerGuard {
    type Target = Server;

    fn deref(&self) -> &Self::Target {
        self.server.as_ref().unwrap()
    }
}

impl DerefMut for ServerGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.server.as_mut().unwrap()
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            // the permit is still held when recycling,
            // so the next acquire will already see the recycled server
            SERVER_POOL.recycle(server);
        }
    }
}

pub(crate) struct ServerPool {
    semaphore: Semaphore,
    free_list: Mutex<VecDeque<Server>>,
}

impl ServerPool {
    const fn new(max_size: usize) -> ServerPool {
        ServerPool {
            semaphore: Semaphore::const_new(max_size),
            free_list: Mutex::new(VecDeque::new()),
        }
    }

    pub(crate) async fn get_async(&'static self) -> Result<ServerGuard, Error> {
        // number of active permits limits the number of servers created
        let permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::Deadlock, err))?;

        // be careful not to lock locks in match - it extends scope of temporaries
        let recycled = self.free_list.lock().unwrap().pop_front();
        let server = match recycled {
            Some(server) => server,
            None => Server::try_new_with_port_async(0).await?,
        };

        Ok(ServerGuard::new(server, permit))
    }

    fn recycle(&self, mut server: Server) {
        server.reset();
        self.free_list.lock().unwrap().push_back(server);
    }
}
