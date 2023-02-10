use crate::Error;
use crate::Server;
use async_trait::async_trait;
use deadpool::managed::{self, Pool};
use lazy_static::lazy_static;

const DEFAULT_POOL_SIZE: usize = 100;

lazy_static! {
    pub(crate) static ref SERVER_POOL: Pool<ServerPool> = ServerPool::new(DEFAULT_POOL_SIZE);
}

pub(crate) struct ServerPool {}

impl ServerPool {
    fn new(max_size: usize) -> Pool<ServerPool> {
        let server_pool = ServerPool {};
        Pool::builder(server_pool)
            .max_size(max_size)
            .build()
            .expect("Could not create server pool")
    }
}

#[async_trait]
impl managed::Manager for ServerPool {
    type Type = Server;
    type Error = Error;

    async fn create(&self) -> Result<Server, Error> {
        Server::try_new_with_port_async(0).await
    }

    async fn recycle(&self, server: &mut Server) -> managed::RecycleResult<Error> {
        server.reset_async().await;
        Ok(())
    }
}
