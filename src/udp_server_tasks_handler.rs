use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use regex::Regex;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

use crate::proxy_toolkit::ProxyToolkit;

const CONNECT_MESSAGE: &'static str = "Connect";
const ACCEPT_RESPONSE: &'static str = "Accept";
const BYE_MESSAGE: &'static str = "BYE";
const BYE_RESPONSE: &'static str = "BYE";
const REPEAT_BATCH_PREXIT: &'static str = "REPEAT_BATCH:";
const BUFFER_SIZE: u32 = 10_000;

pub struct UdpServerTasksHandler {
    pub request_receiver: Receiver<(String, SocketAddr)>,
    pub response_sender: Sender<(Vec<u8>, SocketAddr)>,
    pub recent_batches: Arc<RwLock<HashMap<(u32, SocketAddr), (u128, Vec<u8>)>>>,
}

impl UdpServerTasksHandler {
    pub async fn start(&mut self) {
        while let Some((message, peer)) = self.request_receiver.recv().await {
            let mut recent_batches = Arc::clone(&self.recent_batches.clone());
            let response_sender = self.response_sender.clone();
            // TODO periodic cleanup
            tokio::spawn(async move {
                match message.as_str() {
                    CONNECT_MESSAGE => {
                        if let Err(e) = response_sender.send((ACCEPT_RESPONSE.as_bytes().to_vec(), peer))
                            .await {
                            println!("Failed sending response back: {}", e);
                        }
                    }
                    BYE_MESSAGE => {
                        if let Err(e) = response_sender.send((BYE_RESPONSE.as_bytes().to_vec(), peer))
                            .await {
                            println!("Failed sending response back: {}", e);
                        }
                    }
                    _ => {
                        if message.starts_with(REPEAT_BATCH_PREXIT) {
                            let re = Regex::new(r"^REPEAT_BATCH:(?P<id>\d+)$").unwrap();
                            println!("The message is {}", message);
                            let cap = re.captures(&message);
                            if let Some(capture) = cap {
                                let id: u32 = capture["id"].parse().unwrap();

                                {
                                    let readable_recent_batches = recent_batches.read().await;
                                    println!("The recent batches count is {}", readable_recent_batches.len());
                                    if let Some((_, batch)) = readable_recent_batches.get(&(id, peer)) {
                                        // If the value is still here, even if the ttl has passed, using it
                                        // TODO report to the client
                                        response_sender
                                            .send((batch.clone(), peer))
                                            .await
                                            .map_err(|e| format!("Failed sending to the queue: {}", e.to_string()))
                                            .unwrap();
                                    } else {
                                        // TODO no such peer found in the recent history
                                    }
                                }
                            } else {
                                // TODO some invalid message was sent
                            }
                        } else {
                            if let Err(e) = Self::process_with_failures_logging_on_server(message, peer, response_sender, recent_batches).await {
                                println!("Failed processing a request, failed reporting to the client: {}", e);
                            }
                        }
                    }
                }
            });
        }
    }

    async fn process_with_failures_logging_on_server(message: String, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, recent_batches: Arc<RwLock<HashMap<(u32, SocketAddr), (u128, Vec<u8>)>>>) -> Result<(), String> {
        if let Err(e) = Self::process_with_failures_reporting_to_client(message, peer, response_sender.clone(), recent_batches).await {
            if let Err(reporting_error) = response_sender
                .send((format!("Failed processing your request: {}", e).as_bytes().to_vec(), peer))
                .await {
                return Err(format!("Failed sending to the queue: {}", reporting_error.to_string()));
            }
        }
        Ok(())
    }

    async fn process_with_failures_reporting_to_client(message: String, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, recent_batches: Arc<RwLock<HashMap<(u32, SocketAddr), (u128, Vec<u8>)>>>) -> Result<(), String> {
        let url = ProxyToolkit::process_message(&message.trim())
            .map_err(|e| format!("Invalid url, can't parse it: {}", e))?;
        let message_to_send = ProxyToolkit::generate_content_to_send(&url).await
            .map_err(|e| format!("Issue while loading the data from target server: {}", e))?;
        println!("Message to send has length {} and the peer is {}", message_to_send.len(), peer);
        Self::send_message_with_batches(message_to_send, peer, response_sender, recent_batches).await
            .map_err(|e| format!("Failure when sending the message back to the client: {}", e))?;
        Ok(())
    }

    async fn send_message_with_batches(message: Vec<u8>, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, recent_batches: Arc<RwLock<HashMap<(u32, SocketAddr), (u128, Vec<u8>)>>>) -> Result<(), String> {
        let body_size = BUFFER_SIZE - 8; // 4 bytes for the current index, 4 bytes for the overall
        let overall_batches_raw: usize = (message.len() + body_size as usize - 1) / body_size as usize;
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

            {
                recent_batches
                    .write()
                    .await
                    .insert((i, peer), (SystemTime::now().add(Duration::new(60 * 5, 0)).duration_since(UNIX_EPOCH).unwrap().as_millis(), current_batch.clone()));
            }

            response_sender
                .send((current_batch.to_vec(), peer))
                .await
                .map_err(|e| format!("Failed sending to the queue: {}", e.to_string()))?;
        }
        Ok(())
    }
}