use crate::Server;
use crate::{Error, ErrorKind};
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, SemaphorePermit};

const DEFAULT_POOL_SIZE: usize = 100;

lazy_static! {
    pub(crate) static ref SERVER_POOL: ServerPool = ServerPool::new(DEFAULT_POOL_SIZE);
}

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
            SERVER_POOL.recycle(server);
        }
    }
}

pub(crate) struct ServerPool {
    max_size: usize,
    created: usize,
    semaphore: Semaphore,
    state: Arc<Mutex<VecDeque<Server>>>,
}

impl ServerPool {
    fn new(max_size: usize) -> ServerPool {
        let created = 0;
        let semaphore = Semaphore::new(max_size);
        let state = Arc::new(Mutex::new(VecDeque::new()));
        ServerPool {
            max_size,
            created,
            semaphore,
            state,
        }
    }

    pub(crate) async fn get_async(&'static self) -> Result<ServerGuard, Error> {
        let permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|err| Error::new_with_context(ErrorKind::Deadlock, err))?;

        let state_mutex = self.state.clone();
        let mut state = state_mutex.lock().unwrap();

        if self.created < self.max_size {
            let server = Server::try_new_with_port_async(0).await?;
            state.push_back(server);
        }

        if let Some(server) = state.pop_front() {
            Ok(ServerGuard::new(server, permit))
        } else {
            Err(Error::new(ErrorKind::ServerBusy))
        }
    }

    fn recycle(&self, mut server: Server) {
        server.reset();
        let state_mutex = self.state.clone();
        let mut state = state_mutex.lock().unwrap();
        state.push_back(server);
    }
}
