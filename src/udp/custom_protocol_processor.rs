pub const HEADERS_BYTES_COUNT: usize = 4 * 2;

pub struct CustomProtocolProcessor {

}

impl CustomProtocolProcessor {
    pub fn add_headers(batch: &[u8], batch_id: u32, overall_batches: u32) -> Vec<u8> {
        let mut current_batch = Vec::new();
        current_batch.extend(u32::to_be_bytes(batch_id));
        current_batch.extend(u32::to_be_bytes(overall_batches));
        current_batch.extend(batch);
        return current_batch;
    }
}
