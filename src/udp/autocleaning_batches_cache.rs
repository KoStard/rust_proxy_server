use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::udp::batches_cache::BatchesCache;

pub struct AutocleaningBatchesCache {
    batches_cache: Arc<RwLock<BatchesCache>>,
}

impl AutocleaningBatchesCache {
    pub fn new() -> Self {
        AutocleaningBatchesCache {
            batches_cache: Arc::new(RwLock::new(BatchesCache::new()))
        }
    }

    /// Cleaning up the cache every 5 minutes. Under high loads this might not be enough and
    /// better monitoring might be required (such as checking the memory usage, etc). Currently going with this.
    /// This can be considered as another improvement opportunity.
    pub fn start_loop(&self) {
        let batches_cache = self.batches_cache.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::new(60 * 5, 0)).await;
                {
                    batches_cache
                        .write()
                        .await
                        .cleanup()
                        .await;
                }
            }
        });
    }

    pub async fn add_batch(&mut self, peer: SocketAddr, batch_id: u32, batch: Vec<u8>) {
        self.batches_cache
            .write()
            .await
            .add_batch(peer, batch_id, batch)
            .await
    }
    pub async fn request_batch(&mut self, peer: SocketAddr, batch_id: u32) -> Option<Vec<u8>> {
        self.batches_cache
            .read()
            .await
            .request_batch(peer, batch_id)
            .await
    }
}
