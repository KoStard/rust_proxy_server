use std::io;
use std::net::SocketAddr;

use tokio::net::TcpListener;

use super::custom_tcp_stream::CustomTcpStream;

pub struct CustomTcpListener {
    listener: TcpListener,
}

impl CustomTcpListener {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(CustomTcpListener {listener})
    }

    pub async fn accept(&self) -> io::Result<CustomTcpStream> {
        let (stream, _) = self.listener.accept().await?;
        Ok(CustomTcpStream::new(stream))
    }
}
