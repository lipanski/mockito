use crate::server::{RemoteMock, State};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::sync::MutexGuard;

#[derive(Debug)]
pub(crate) enum Command {
    CreateMock {
        remote_mock: RemoteMock,
        response_sender: oneshot::Sender<bool>,
    },
    GetMockHits {
        mock_id: String,
        response_sender: oneshot::Sender<Option<usize>>,
    },
    RemoveMock {
        mock_id: String,
        response_sender: oneshot::Sender<bool>,
    },
    GetLastUnmatchedRequest {
        response_sender: oneshot::Sender<Option<String>>,
    },
    Noop,
}

impl Command {
    pub(crate) async fn create_mock(sender: &Sender<Command>, remote_mock: RemoteMock) -> bool {
        let (response_sender, response_receiver) = oneshot::channel();

        let cmd = Command::CreateMock {
            remote_mock,
            response_sender,
        };

        let _send = sender.send(cmd).await;
        response_receiver.await.unwrap_or(false)
    }

    pub(crate) async fn get_mock_hits(sender: &Sender<Command>, mock_id: String) -> Option<usize> {
        let (response_sender, response_receiver) = oneshot::channel();

        let cmd = Command::GetMockHits {
            mock_id,
            response_sender,
        };

        let _send = sender.send(cmd).await;
        response_receiver.await.unwrap()
    }

    /// This method has to be sync since it's called from Drop.
    pub(crate) fn remove_mock(sender: &Sender<Command>, mock_id: String) -> bool {
        let (response_sender, response_receiver) = oneshot::channel();

        let cmd = Command::RemoveMock {
            mock_id,
            response_sender,
        };

        let _send = sender.blocking_send(cmd);
        response_receiver.blocking_recv().unwrap_or(false)
    }

    pub(crate) async fn get_last_unmatched_request(sender: &Sender<Command>) -> Option<String> {
        let (response_sender, response_receiver) = oneshot::channel();

        let cmd = Command::GetLastUnmatchedRequest { response_sender };

        let _send = sender.send(cmd).await;
        response_receiver.await.unwrap_or(None)
    }

    pub async fn handle(cmd: Command, mut state: MutexGuard<'_, State>) {
        match cmd {
            Command::CreateMock {
                remote_mock,
                response_sender,
            } => {
                state.mocks.push(remote_mock);

                let _send = response_sender.send(true);
            }
            Command::GetMockHits {
                mock_id,
                response_sender,
            } => {
                let hits: Option<usize> = state
                    .mocks
                    .iter()
                    .find(|remote_mock| remote_mock.inner.id == mock_id)
                    .map(|remote_mock| remote_mock.inner.hits);

                let _send = response_sender.send(hits);
            }
            Command::RemoveMock {
                mock_id,
                response_sender,
            } => {
                if let Some(pos) = state
                    .mocks
                    .iter()
                    .position(|remote_mock| remote_mock.inner.id == mock_id)
                {
                    state.mocks.remove(pos);
                }

                let _send = response_sender.send(true);
            }
            Command::GetLastUnmatchedRequest { response_sender } => {
                let last_unmatched_request = state.unmatched_requests.last_mut();

                let label = match last_unmatched_request {
                    Some(req) => Some(req.to_string().await),
                    None => None,
                };

                let _send = response_sender.send(label);
            }
            Command::Noop => {}
        }
    }
}
