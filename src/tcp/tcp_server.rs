use crate::{proxy_logic::ProxyLogic, toolkit};

use super::{custom_tcp_listener::CustomTcpListener, custom_tcp_stream::CustomTcpStream};

const CONNECT_MESSAGE: &'static str = "Connect";
const ACCEPT_RESPONSE: &'static str = "Accept";
const BYE_MESSAGE: &'static str = "BYE";
const BYE_RESPONSE: &'static str = "BYE";

pub struct TcpServer {}

impl TcpServer {
    pub async fn start(
        &self,
        listener: CustomTcpListener,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting the TCP server...");
        loop {
            let stream = listener.accept().await?;
            tokio::spawn(Self::handle_tcp_client(stream));
        }
    }

    async fn handle_tcp_client(mut stream: CustomTcpStream) {
        let res = Self::process_communication(&mut stream).await;
        if let Err(e) = res {
            if let Err(reporting_error) = stream
                .write_full_message(format!("Error occurred: {}\n", e).as_bytes())
                .await
            {
                println!(
                    "Failed reporting about exception {}, got another exception: {}",
                    e, reporting_error
                );
            }
        }
    }

    /// Is closing the connection in case of failures, can be improved
    async fn process_communication(stream: &mut CustomTcpStream) -> Result<(), String> {
        Self::handle_greeting(stream).await?;
        Self::handle_the_main_message(stream).await?;
        Self::handle_bye(stream).await?;
        Ok(())
    }

    async fn handle_greeting(stream: &mut CustomTcpStream) -> Result<(), String> {
        let message = toolkit::bytes_to_string(&stream.read_full_tcp_message().await?);
        if message == CONNECT_MESSAGE {
            stream.write_full_message(ACCEPT_RESPONSE.as_bytes()).await
        } else {
            Err("Expected connect message".to_owned())
        }
    }

    async fn handle_the_main_message(stream: &mut CustomTcpStream) -> Result<(), String> {
        // TODO url validation
        let message = toolkit::bytes_to_string(&stream.read_full_tcp_message().await?);
        let url = ProxyLogic::process_message(&message)?;
        let message_to_send = ProxyLogic::generate_content_to_send(&url).await?;
        stream.write_full_message(message_to_send.as_slice()).await
    }

    async fn handle_bye(stream: &mut CustomTcpStream) -> Result<(), String> {
        let message = toolkit::bytes_to_string(&stream.read_full_tcp_message().await?);
        if message == BYE_MESSAGE {
            stream.write_full_message(BYE_RESPONSE.as_bytes()).await
        } else {
            return Err("Expected bye message".to_owned());
        }
    }
}
