use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::proxy_toolkit::ProxyToolkit;

pub struct TcpServer {
    pub listener: TcpListener,
}

const MAX_BATCH_SIZE: usize = 500;
const MAX_MESSAGE_SIZE: usize = 10000;
const CONNECT_MESSAGE: &'static str = "Connect";
const ACCEPT_RESPONSE: &'static str = "Accept";
const BYE_MESSAGE: &'static str = "BYE";
const BYE_RESPONSE: &'static str = "BYE";

impl TcpServer {
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            tokio::spawn(Self::handle_tcp_client(stream));
        }
    }

    async fn handle_tcp_client(mut stream: TcpStream) {
        let res = Self::process_communication(&mut stream).await;
        if let Err(e) = res {
            if let Err(reporting_error) = Self::send_tcp_message(&mut stream, format!("Error occurred: {}\n", e).as_bytes()).await {
                println!("Failed reporting about exception {}, got another exception: {}", e, reporting_error);
            }
        }
    }

    /// Is closing the connection in case of failures, can be improved
    async fn process_communication(stream: &mut TcpStream) -> Result<(), String> {
        let message = Self::response_to_string(&Self::load_tcp_message(stream).await?);
        if message == CONNECT_MESSAGE {
            Self::send_tcp_message(stream, ACCEPT_RESPONSE.as_bytes()).await?;
        } else {
            return Err("Expected connect message".to_owned());
        }

        // TODO url validation
        let message = Self::response_to_string(&Self::load_tcp_message(stream).await?);
        let url = ProxyToolkit::process_message(&message)?;
        let message_to_send = ProxyToolkit::generate_content_to_send(&url).await?;
        Self::send_tcp_message(stream, message_to_send.as_slice()).await?;

        let message = Self::response_to_string(&Self::load_tcp_message(stream).await?);
        if message == BYE_MESSAGE {
            Self::send_tcp_message(stream, BYE_RESPONSE.as_bytes()).await?;
        } else {
            return Err("Expected bye message".to_owned());
        }
        Ok(())
    }

    async fn send_tcp_message(stream: &mut TcpStream, message: &[u8]) -> Result<(), String> {
        let buf = Self::add_headers(message);
        let mut index = 0;
        while index < buf.len() {
            let count = stream.write(&buf[index..])
                .await
                .map_err(|e| format!("Failed sending TCP message: {}", e))?;
            index += count;
        }
        Ok(())
    }

    fn response_to_string(response: &[u8]) -> String {
        return String::from_utf8_lossy(response).to_string();
    }

    /// Using custom protocol here
    /// First 4 bytes should be responsible for showing the length of the request
    async fn load_tcp_message(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
        println!("Reading TCP message from {:?}", stream);
        let mut overall_message = Vec::new();
        let (overall_length, current_body) = Self::tcp_read_with_headers(stream).await?;
        if overall_length as usize > MAX_MESSAGE_SIZE {
            return Err(format!("The maximum message size is {}, you gave bigger message", MAX_MESSAGE_SIZE));
        }
        overall_message.extend(current_body);
        while overall_message.len() < overall_length as usize {
            overall_message.extend(Self::one_tcp_read(stream).await?);
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

    /// Will return the length of the message (u32) and the rest of the body that was consumed
    async fn tcp_read_with_headers(stream: &mut TcpStream) -> Result<(u32, Vec<u8>), String> {
        let mut initial_message = Vec::new();
        while initial_message.len() < 4 {
            initial_message.extend(Self::one_tcp_read(stream).await?);
        }
        Ok(Self::parse_headers(initial_message))
    }

    fn add_headers(message: &[u8]) -> Vec<u8> {
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

    fn parse_headers(message: Vec<u8>) -> (u32, Vec<u8>) {
        (u32::from_be_bytes([message[0], message[1], message[2], message[3]]),
                message[4..].to_vec())
    }

    async fn one_tcp_read(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
        // TODO check if will block if not enough message was sent
        let mut buffer = [0; MAX_BATCH_SIZE];
        let count = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if count == 0 {
            Err("Issue with the TCP read, got 0 bytes".to_owned())
        } else {
            Ok(buffer[..count].to_vec())
        }
    }
}
