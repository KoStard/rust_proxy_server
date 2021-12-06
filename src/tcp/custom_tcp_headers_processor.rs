pub const HEADERS_LENGTH: usize = 4;

pub struct CustomTcpHeadersProcessor {}

/// First 4 bytes will be the header for showing the length of the content
impl CustomTcpHeadersProcessor {
    pub fn parse_headers(message: Vec<u8>) -> (u32, Vec<u8>) {
        (
            u32::from_be_bytes([message[0], message[1], message[2], message[3]]),
            message[4..].to_vec(),
        )
    }

    pub fn add_headers(message: &[u8]) -> Vec<u8> {
        let length = message.len();
        if length > u32::MAX as usize {
            panic!("Maximum allowed length is {}", u32::MAX);
        }
        let length_bytes = (length as u32).to_be_bytes();
        let mut new_message = Vec::new();
        new_message.extend(length_bytes);
        new_message.extend(message);
        return new_message;
    }
}
