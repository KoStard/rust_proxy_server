pub fn bytes_to_string(response: &[u8]) -> String {
    String::from_utf8_lossy(response).to_string()
}