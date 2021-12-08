use std::net::SocketAddr;
use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

use crate::toolkit;
use crate::udp::custom_protocol_processor::CustomProtocolProcessor;
use crate::udp::custom_udp_socket::CustomUdpSocket;

pub struct UdpServer {
    socket: CustomUdpSocket,
    request_sender: Sender<(String, SocketAddr)>,
    response_receiver: Receiver<(Vec<u8>, SocketAddr)>,
    idle_loop_counter: u32,
}

impl UdpServer {
    pub fn new(socket: CustomUdpSocket, request_sender: Sender<(String, SocketAddr)>, response_receiver: Receiver<(Vec<u8>, SocketAddr)>) -> Self {
        UdpServer {
            socket,
            request_sender,
            response_receiver,
            idle_loop_counter: 0,
        }
    }
    pub async fn start(&mut self) {
        println!("UDP Server starting");
        loop {
            self.one_loop().await;
        }
    }

    async fn one_loop(&mut self) {
        let mut request_received = false;
        let mut response_received = false;

        // This might result in never-decreasing backlog, can be improved in the future
        match self.socket.try_recv_from_and_validate() {
            Ok(Some((bytes, peer))) => {
                // Maybe we can trim the message, but might have unexpected side effects
                let message = toolkit::bytes_to_string(bytes.as_slice());
                if let Err(e) = self.request_sender.send((message, peer)).await {
                    self.report_failure(format!("Failed sending message to the requests queue: {}", e), peer).await;
                }
                request_received = true;
            }
            Ok(None) => {
                // Did not receive any message, no problem
            }
            Err((peer, exception_message)) => {
                self.report_failure(exception_message, peer).await;
            }
        }

        while let Ok((buffer, peer)) = self.response_receiver.try_recv() {
            response_received = true;
            println!("Sending {} bytes", buffer.len());
            if let Err(exception_message) = self.socket.send_to(buffer.as_slice(), &peer).await {
                println!("{}", exception_message);
                // self.report_failure(exception_message, peer).await;
                // This might harm more
            }
        }
        // Sleeping the thread to save resources in idle state
        if !request_received && !response_received {
            if self.idle_loop_counter < 50 {
                self.idle_loop_counter += 1;
            } else {
                sleep(Duration::from_millis(25)).await;
            }
        } else {
            self.idle_loop_counter = 0;
        }
    }

    /// Trying to report failure to the client, if even the reporting fails, just logging
    async fn report_failure(&self, message: String, peer: SocketAddr) {
        if let Err(reporting_failure_message) = self.socket.send_to(CustomProtocolProcessor::add_headers(message.as_bytes(), 0, 1).as_slice(), &peer).await {
            println!("Failed reporting to the client with message {} about another failure: {}", reporting_failure_message, message);
        }
    }
}
