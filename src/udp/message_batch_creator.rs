pub struct MessageBatchCreator {
    batch_size: usize,
}

impl MessageBatchCreator {
    pub fn new(batch_size: usize) -> Self {
        MessageBatchCreator {
            batch_size
        }
    }

    pub fn break_message(&self, message: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
        let overall_batches_raw: usize = (message.len() + self.batch_size - 1) / self.batch_size;
        if overall_batches_raw > u32::MAX.try_into().unwrap() {
            return Err("Very long message, can't break into batches".to_owned());
        }
        let overall_batches = overall_batches_raw as u32;
        let slc = message.as_slice();

        let mut all_batches = Vec::new();

        for i in 0..overall_batches {
            all_batches.push(slc[(i * self.batch_size as u32) as usize..(((i + 1) * self.batch_size as u32) as usize).min(message.len())].to_vec());
        }

        Ok(all_batches)
    }
}
