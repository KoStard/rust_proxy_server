mod tcp;
mod toolkit;

use std::collections::HashMap;
use std::sync::Arc;
use tcp::custom_tcp_listener::CustomTcpListener;
use tcp::tcp_server::TcpServer;
use tokio::net::{UdpSocket};
use tokio::sync::{mpsc, RwLock};

mod udp_server;
mod udp_server_tasks_handler;
mod proxy_logic;

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
    let tcp_listener = CustomTcpListener::new("0.0.0.0:4000".parse().unwrap()).await?;
    promises.push(tokio::spawn(async move {
        TcpServer {}.start(tcp_listener).await.expect("TCP server failed running");
    }));

    futures::future::join_all(promises).await;
    Ok(())
}

// TODO ip spoofing

