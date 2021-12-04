use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

pub struct UdpServer {
    pub socket: UdpSocket,
    pub request_sender: Sender<(String, SocketAddr)>,
    pub response_receiver: Receiver<(Vec<u8>, SocketAddr)>,
}

impl UdpServer {
    pub async fn start(&mut self) {
        println!("Starting the server");
        loop {
            self.one_loop().await;
        }
    }

    pub async fn one_loop(&mut self) {
        const MAX_BATCH_SIZE: usize = 10000;
        const MAX_MESSAGE_SIZE: usize = 10000;
        let mut buffer = [0; MAX_BATCH_SIZE];
        let mut request_received = false;
        let mut response_received = false;
        if let Ok((size, peer)) = self.socket.try_recv_from(&mut buffer) {
            let message = String::from_utf8_lossy(&buffer[..size])
                .to_string();
            let trimmed_message = message.trim();

            self.request_sender.send((trimmed_message.to_owned(), peer)).await.unwrap();
            request_received = true;
        }

        while let Ok((buffer, peer)) = self.response_receiver.try_recv() {
            self.socket.send_to(buffer.as_slice(), peer).await.unwrap();
            response_received = true;
        }

        if !request_received && !response_received {
            // To save resources
            // println!("Didn't receive any message, sleeping");
            // TODO decrease this number in prod
            sleep(Duration::from_millis(100)).await;
        }
    }
}