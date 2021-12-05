use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, RwLock};

mod udp_server;
mod udp_server_tasks_handler;
mod proxy_toolkit;
mod tcp_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Allow defining the port from a CLI
    let mut promises = vec![];

    // Setting up UDP server
    let socket = UdpSocket::bind("0.0.0.0:4000").await?;
    let (request_sender, request_receiver) = mpsc::channel(100);
    let (response_sender, response_receiver) = mpsc::channel(100);
    promises.push(tokio::spawn(async move {
        udp_server::UdpServer {
            socket,
            request_sender,
            response_receiver,
        }.start().await;
    }));

    promises.push(tokio::spawn(async move {
        udp_server_tasks_handler::UdpServerTasksHandler {
            request_receiver,
            response_sender,
            recent_batches: Arc::new(RwLock::new(HashMap::new())),
        }.start().await;
    }));

    // Setting up TCP server
    let tcp_listener = TcpListener::bind("0.0.0.0:4000").await?;
    promises.push(tokio::spawn(async move {
        tcp_server::TcpServer {
            listener: tcp_listener
        }.start().await.expect("TCP server failed running");
    }));

    futures::future::join_all(promises).await;
    Ok(())
}

// TODO ip spoofing

