use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

use crate::proxy_logic::ProxyLogic;
use crate::udp::autocleaning_batches_cache::AutocleaningBatchesCache;
use crate::udp::batch_repeat_helper::{get_batch_id_for_repeat, is_batch_repeat_request};
use crate::udp::custom_protocol_processor::{CustomProtocolProcessor, HEADERS_BYTES_COUNT};
use crate::udp::message_batch_creator::MessageBatchCreator;

const CONNECT_MESSAGE: &'static str = "Connect";
const ACCEPT_RESPONSE: &'static str = "Accept";
const BYE_MESSAGE: &'static str = "BYE";
const BYE_RESPONSE: &'static str = "BYE";
const BUFFER_SIZE: usize = 500;

pub struct UdpServerTasksHandler {
    request_receiver: Receiver<(String, SocketAddr)>,
    response_sender: Sender<(Vec<u8>, SocketAddr)>,
    autocleaning_batches_cache: Arc<RwLock<AutocleaningBatchesCache>>,
}

impl UdpServerTasksHandler {
    pub fn new(request_receiver: Receiver<(String, SocketAddr)>, response_sender: Sender<(Vec<u8>, SocketAddr)>) -> Self {
        UdpServerTasksHandler {
            request_receiver,
            response_sender,
            autocleaning_batches_cache: Arc::new(RwLock::new(AutocleaningBatchesCache::new())),
        }
    }

    pub async fn start(&mut self) {
        println!("Starting UDP server tasks handler...");
        {
            self.autocleaning_batches_cache.read().await.start_loop();
        }
        while let Some((message, peer)) = self.request_receiver.recv().await {
            let response_sender = self.response_sender.clone();
            let autocleaning_batches_cache = self.autocleaning_batches_cache.clone();
            tokio::spawn(async move {
                let message_str = message.as_str();
                match message_str {
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
                        if is_batch_repeat_request(message_str) {
                            if let Some(id) = get_batch_id_for_repeat(message_str) {
                                let bytes_to_send = autocleaning_batches_cache
                                    .write()
                                    .await
                                    .request_batch(peer, id)
                                    .await
                                    .unwrap_or(format!("Couldn't get the requested batch with ID {}", id).as_bytes().to_vec());
                                if let Err(e) = response_sender
                                    .send((bytes_to_send, peer))
                                    .await {
                                    println!("Failed sending to the response sender, the client might retry...\n{}", e);
                                }
                            } else {
                                println!("Invalid message was send, couldn't process. If this was UDP issue, the client will retry.")
                            }
                        } else {
                            if let Err(e) = Self::process_with_failures_logging_on_server(message, peer, response_sender, autocleaning_batches_cache).await {
                                println!("Failed processing a request, failed reporting to the client: {}", e);
                            }
                        }
                    }
                }
            });
        }
    }

    async fn process_with_failures_logging_on_server(message: String, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, autocleaning_batches_cache: Arc<RwLock<AutocleaningBatchesCache>>) -> Result<(), String> {
        if let Err(e) = Self::process_with_failures_reporting_to_client(message, peer, response_sender.clone(), autocleaning_batches_cache.clone()).await {
            if let Err(reporting_error) = Self::send_message_with_batches(format!("Failed processing your request: {}", e).into_bytes(), peer, response_sender, autocleaning_batches_cache)
                .await {
                return Err(format!("Failed sending to the queue: {}", reporting_error.to_string()));
            }
        }
        Ok(())
    }

    async fn process_with_failures_reporting_to_client(message: String, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, autocleaning_batches_cache: Arc<RwLock<AutocleaningBatchesCache>>) -> Result<(), String> {
        let url = ProxyLogic::process_message(&message.trim())
            .map_err(|e| format!("Invalid url, can't parse it: {}", e))?;
        let message_to_send = ProxyLogic::generate_content_to_send(&url).await
            .map_err(|e| format!("Issue while loading the data from target server: {}", e))?;
        println!("Message to send has length {} and the peer is {}", message_to_send.len(), peer);
        Self::send_message_with_batches(message_to_send, peer, response_sender, autocleaning_batches_cache).await
            .map_err(|e| format!("Failure when sending the message back to the client: {}", e))?;
        Ok(())
    }

    async fn send_message_with_batches(message: Vec<u8>, peer: SocketAddr, response_sender: Sender<(Vec<u8>, SocketAddr)>, autocleaning_batches_cache: Arc<RwLock<AutocleaningBatchesCache>>) -> Result<(), String> {
        let message_batch_creator = MessageBatchCreator::new(BUFFER_SIZE - HEADERS_BYTES_COUNT);
        let batches = message_batch_creator
            .break_message(message)?;
        let mut index = 0;
        let batches_count = batches.len();
        for batch in batches.iter() {
            let current_batch = CustomProtocolProcessor::add_headers(batch.as_slice(), index as u32, batches_count as u32);
            {
                autocleaning_batches_cache
                    .write()
                    .await
                    .add_batch(peer, index as u32, current_batch.clone())
                    .await;
            }

            // If publishing fails, we should still try to send the next batches, as the client will retry for the failed ones
            if let Err(e) = response_sender
                .send((current_batch.to_vec(), peer))
                .await {
                println!("Failed sending batch {} to the queue: {}", index, e.to_string());
            }

            index += 1;
        }
        Ok(())
    }
}
