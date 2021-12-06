use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

use super::custom_tcp_headers_processor::{CustomTcpHeadersProcessor, HEADERS_LENGTH};

// TODO check the constants
const MAX_BATCH_SIZE: usize = 100;
const MAX_MESSAGE_SIZE: usize = 10000;

pub struct CustomTcpStream {
    stream: TcpStream
}

impl CustomTcpStream {
    pub fn new(stream: TcpStream) -> Self {
        CustomTcpStream {stream}
    }

    pub async fn read_full_tcp_message(&mut self) -> Result<Vec<u8>, String> {
        let mut overall_message = Vec::new();
        let (overall_length, current_body) = self.first_tcp_read_with_headers().await?;
        if overall_length as usize > MAX_MESSAGE_SIZE {
            return Err(format!("The maximum message size is {}, you gave bigger message", MAX_MESSAGE_SIZE));
        }
        overall_message.extend(current_body);
        while overall_message.len() < overall_length as usize {
            overall_message.extend(self.raw_tcp_read().await?);
        }

        // This might be the continuation of the next message actually, we can improve this by
        // creating buffer that will fit only the current message and not more, but for current
        // usecase it's not possible, as the client will be waiting for response after sending the
        // URL
        if overall_message.len() > overall_length as usize {
            Ok(overall_message[..overall_length as usize].to_vec())
        } else {
            Ok(overall_message)
        }
    }

    async fn first_tcp_read_with_headers(&mut self) -> Result<(u32, Vec<u8>), String> {
        let mut initial_message = Vec::new();
        while initial_message.len() < HEADERS_LENGTH {
            initial_message.extend(self.raw_tcp_read().await?);
        }
        Ok(CustomTcpHeadersProcessor::parse_headers(initial_message))
    }

    async fn raw_tcp_read(&mut self) -> Result<Vec<u8>, String> {
        let mut buffer = [0; MAX_BATCH_SIZE];
        let count = self.stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if count == 0 {
            Err("Issue with the TCP read, got 0 bytes".to_owned())
        } else {
            Ok(buffer[..count].to_vec())
        }
    }

    pub async fn write_full_message(&mut self, message: &[u8]) -> Result<(), String> {
        let buf = CustomTcpHeadersProcessor::add_headers(message);
        let mut index = 0;
        while index < buf.len() {
            let count = self.stream.write(&buf[index..])
                .await
                .map_err(|e| format!("Failed sending TCP message: {}", e))?;
            index += count;
        }
        Ok(())
    }
}
