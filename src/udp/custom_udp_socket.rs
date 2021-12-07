use std::net::SocketAddr;
use tokio::net::UdpSocket;

pub struct CustomUdpSocket {
    socket: UdpSocket,
}

// Using max message size as 10000, as the target server won't even be able to handle even longer URLs.
// Source: https://stackoverflow.com/questions/417142/what-is-the-maximum-length-of-a-url-in-different-browsers
const MAX_MESSAGE_SIZE: usize = 10000;
/// Using MAX_MESSAGE_SIZE + 1, so that we can understand if the actual message is longer than the maximum or not
const MAX_BATCH_SIZE: usize = MAX_MESSAGE_SIZE + 1;

impl CustomUdpSocket {
    pub fn new(socket: UdpSocket) -> Self {
        CustomUdpSocket { socket }
    }

    pub fn try_recv_from_and_validate(&self) -> Result<Option<(Vec<u8>, SocketAddr)>, (SocketAddr, String)> {
        let mut buffer = [0; MAX_BATCH_SIZE];
        if let Ok((size, peer)) = self.socket.try_recv_from(&mut buffer) {
            if size > MAX_MESSAGE_SIZE {
                return Err((peer, format!("Invalid message length, max is {}", MAX_MESSAGE_SIZE)));
            }
            Ok(Some((buffer[..size].to_vec(), peer)))
        } else {
            Ok(None)
        }
    }

    pub async fn send_to(&self, bytes: &[u8], peer: &SocketAddr) -> Result<(), String> {
        let resp = self.socket.send_to(bytes, peer).await;
        match resp {
            Ok(c) => {
                if c == bytes.len() {
                    Ok(())
                } else {
                    panic!("Udp socket did not write everything down, not implemented case");
                }
            }
            Err(e) => {
                Err(format!("Error sending to {}, got exception {}", peer, e))
            }
        }
    }
}
