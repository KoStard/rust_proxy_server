use std::net::SocketAddr;
use tokio::sync::mpsc::{Receiver, Sender};
use crate::ProxyToolkit::ProxyToolkit;

const CONNECT_MESSAGE: &'static str = "Connect";
const ACCEPT_RESPONSE: &'static str = "Accept";
const BYE_MESSAGE: &'static str = "BYE";
const BYE_RESPONSE: &'static str = "BYE";
const BUFFER_SIZE: u32 = 10_000;

pub struct UdpServerTasksHandler {
    pub request_receiver: Receiver<(String, SocketAddr)>,
    pub response_sender: Sender<(Vec<u8>, SocketAddr)>,
}

impl UdpServerTasksHandler {
    // TODO: Forward exceptions to the client
    pub async fn start(&mut self) {
        // TODO check if you can trim the message directly with the bytes count from response
        while let Some((message, peer)) = self.request_receiver.recv().await {
            let response_sender = self.response_sender.clone();
            tokio::spawn(async move {
                match message.as_str() {
                    CONNECT_MESSAGE => {
                        response_sender.send((ACCEPT_RESPONSE.as_bytes().to_vec(), peer))
                            .await
                            .unwrap();
                    }
                    BYE_MESSAGE => {
                        response_sender.send((BYE_RESPONSE.as_bytes().to_vec(), peer))
                            .await
                            .unwrap();
                    }
                    _ => {
                        let url = ProxyToolkit::process_message(&message
                            .trim_end_matches(|c: char| c.eq(&'\u{0}'))
                            .trim_end()).unwrap();
                        let message_to_send = ProxyToolkit::generate_content_to_send(&url).await.unwrap();
                        println!("Message to send has length {} and the peer is {}", message_to_send.len(), peer);
                        Self::send_message_with_batches(message_to_send, peer, response_sender).await.unwrap();
                        // response_sender.send((message_to_send, peer)).await.unwrap();
                    }
                }
            });
        }
    }

    async fn send_message_with_batches(message: Vec<u8>, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>) -> Result<(), String> {
        let body_size = BUFFER_SIZE - 8; // 4 bytes for the current index, 4 bytes for the overall
        let overall_batches_raw: usize = ((message.len() + body_size as usize - 1) / body_size as usize);
        if overall_batches_raw > u32::MAX.try_into().unwrap() {
            return Err("Very long message, can't send".to_owned());
        }
        let slc = message.as_slice();
        let overall_batches: u32 = overall_batches_raw as u32;
        for i in 0..overall_batches {
            println!("Sending batch N{} with UDP to {}", i, peer);
            let current_body = &slc[(i * body_size) as usize..(((i + 1) * body_size) as usize).min(message.len())];
            let mut current_batch = Vec::new();
            current_batch.extend(u32::to_be_bytes(i));
            current_batch.extend(u32::to_be_bytes(overall_batches));
            current_batch.extend(current_body);
            response_sender
                .send((current_batch.to_vec(), peer))
                .await
                .unwrap();
        }
        Ok(())
    }
}