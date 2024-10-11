use crate::settings::Settings;
use linkme::distributed_slice;

#[distributed_slice]
pub static STARTERS: [fn(settings: &Settings, rx: WorkerHandle) -> anyhow::Result<()>];

pub struct WorkerHandle {
    pub shutdown_rx: Option<tokio::sync::broadcast::Receiver<()>>,
    pub wait_worker_tx: tokio::sync::oneshot::Sender<()>,
}

impl WorkerHandle {
    pub fn new(
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
        wait_worker_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Self {
        Self {
            shutdown_rx: Some(shutdown_rx),
            wait_worker_tx,
        }
    }

    pub fn done(self) -> anyhow::Result<()> {
        self.wait_worker_tx
            .send(())
            .map_err(|_| anyhow::anyhow!("failed to send done signal"))
    }

    pub fn signal(&mut self) -> tokio::sync::broadcast::Receiver<()> {
        self.shutdown_rx.take().unwrap()
    }
}
