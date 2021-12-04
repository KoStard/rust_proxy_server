mod UdpServer;
mod UdpServerTasksHandler;
mod ProxyToolkit;
mod TcpServer;

use std::{io::{Read, Bytes}};
use std::net::SocketAddr;
use std::time::Duration;
use futures::StreamExt;

use regex::Regex;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut promises = vec![];

    // Setting up UDP server
    let socket = UdpSocket::bind("127.0.0.1:4000").await.unwrap();
    let (mut request_sender, mut request_receiver) = mpsc::channel(100);
    let (mut response_sender, mut response_receiver) = mpsc::channel(100);
    promises.push(tokio::spawn(async move {
        UdpServer::UdpServer {
            socket,
            request_sender,
            response_receiver,
        }.start().await;
    }));

    promises.push(tokio::spawn(async move {
        UdpServerTasksHandler::UdpServerTasksHandler {
            request_receiver,
            response_sender,
        }.start().await;
    }));

    // Setting up TCP server
    let tcp_listener = TcpListener::bind("127.0.0.1:4000").await?;
    promises.push(tokio::spawn(async move {
        TcpServer::TcpServer {
            listener: tcp_listener
        }.start().await;
    }));

    // create_udp_server().await.unwrap();
    // tokio::spawn(async move { create_udp_server().await.unwrap() });
    // tokio::spawn(async move { create_tcp_server().await.unwrap() });
    // loop {}

    futures::future::join_all(promises).await;
    Ok(())
}

// TODO ip spoofing

