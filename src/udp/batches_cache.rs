use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

// TODO documentation
pub struct BatchesCache {
    recent_batches: Arc<RwLock<HashMap<(u32, SocketAddr), (SystemTime, Vec<u8>)>>>,
}

impl BatchesCache {
    pub fn new() -> Self {
        BatchesCache { recent_batches: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn add_batch(&mut self, peer: SocketAddr, batch_id: u32, batch: Vec<u8>) {
        self.recent_batches
            .write()
            .await
            .insert((batch_id, peer), (SystemTime::now().add(Duration::new(60 * 5, 0)), batch));
    }

    pub async fn request_batch(&self, peer: SocketAddr, batch_id: u32) -> Option<Vec<u8>> {
        self.recent_batches
            .read()
            .await
            .get(&(batch_id, peer))
            .map(|v| v.1.clone())
    }

    pub async fn cleanup(&mut self) {
        let mut recent_batches = self.recent_batches.write().await;
        println!("Cleaning up the batches cache, currently has {} batches", recent_batches.len());
        let current = SystemTime::now();
        let mut remove_queue = vec![];
        for (k, (ttl, _)) in recent_batches.iter() {
            if *ttl < current {
                remove_queue.push(*k);
            }
        }
        remove_queue
            .iter()
            .for_each(|k| { recent_batches.remove(k).unwrap(); });
        println!("Removed {} items from the batches cache", remove_queue.len());
    }
}
