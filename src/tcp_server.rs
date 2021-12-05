use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use crate::proxy_toolkit::ProxyToolkit;

pub struct TcpServer {
    pub listener: TcpListener,
}

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
            if let Err(reporting_error) = stream.write(format!("Error occurred: {}\n", e).as_bytes()).await {
                println!("Failed reporting about exception {}, got another exception: {}", e, reporting_error);
            }
        }
    }

    async fn process_communication(stream: &mut TcpStream) -> Result<(), String> {
        println!("TCP Connection established with {:?}", stream);
        const MAX_BATCH_SIZE: usize = 500;
        const MAX_MESSAGE_SIZE: usize = 10000;
        let mut overall_message = String::new();
        loop {
            let mut buffer = [0; MAX_BATCH_SIZE];
            let count = stream.try_read(&mut buffer).map_err(|e| e.to_string())?;
            let message = String::from_utf8_lossy(&buffer[..count]);
            let fully_trimmed_message = message.trim_end();
            overall_message.push_str(fully_trimmed_message);

            if overall_message.len() > MAX_MESSAGE_SIZE {
                return Err(format!(
                    "The maximum length of input message was exceeded. The limit is {}",
                    MAX_MESSAGE_SIZE
                ));
            }

            if count == 0
                || count < MAX_BATCH_SIZE {
                // TODO think about this
                println!("Finished the message {}.", overall_message);
                break;
            }
        }

        let url = ProxyToolkit::process_message(&overall_message)?;

        // TODO url validation

        let message_to_send = ProxyToolkit::generate_content_to_send(&url).await?;
        if let Err(e) = stream.write(message_to_send.as_slice()).await {
            println!("Failed sending message: {}", e);
        }

        Ok(())
    }
}
