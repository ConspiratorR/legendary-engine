//! Low-level TCP and UDP socket wrappers for game networking.

use std::collections::VecDeque;
use std::net::ToSocketAddrs;
use std::sync::Mutex;
use std::time::Duration;

use thiserror::Error;

/// Errors that can occur during socket operations.
#[derive(Debug, Error)]
pub enum SocketError {
    #[error("failed to bind socket: {0}")]
    BindFailed(std::io::Error),
    #[error("failed to send data: {0}")]
    SendFailed(std::io::Error),
    #[error("failed to receive data: {0}")]
    ReceiveFailed(std::io::Error),
    #[error("connection closed")]
    ConnectionClosed,
    #[error("operation timed out")]
    Timeout,
    #[error("invalid address: {0}")]
    InvalidAddress(String),
}

/// Transport protocol selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Both,
}

/// Configuration for network socket binding.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub port: u16,
    pub protocol: Protocol,
    pub max_packet_size: usize,
    pub timeout_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 0,
            protocol: Protocol::Udp,
            max_packet_size: 65536,
            timeout_ms: 5000,
        }
    }
}

/// A received network packet.
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub data: Vec<u8>,
    pub sender_addr: String,
    pub timestamp: u64,
}

/// Thread-safe queue for received packets.
#[derive(Debug)]
pub struct PacketQueue {
    queue: Mutex<VecDeque<NetworkPacket>>,
}

impl PacketQueue {
    /// Create a new empty packet queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Push a packet to the back of the queue.
    pub fn push(&self, packet: NetworkPacket) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(packet);
    }

    /// Pop a packet from the front of the queue.
    pub fn pop(&self) -> Option<NetworkPacket> {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    /// Get the number of packets in the queue.
    pub fn len(&self) -> usize {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}

impl Default for PacketQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// UDP socket wrapper.
pub struct UdpSocket {
    socket: std::net::UdpSocket,
}

impl UdpSocket {
    /// Bind a UDP socket to the given address (e.g. "0.0.0.0:0").
    pub fn bind(addr: &str) -> Result<Self, SocketError> {
        let socket = std::net::UdpSocket::bind(addr).map_err(SocketError::BindFailed)?;
        Ok(Self { socket })
    }

    /// Send data to a specific address.
    pub fn send_to(&self, data: &[u8], addr: &str) -> Result<usize, SocketError> {
        let dest = addr
            .to_socket_addrs()
            .map_err(|e| SocketError::InvalidAddress(e.to_string()))?
            .next()
            .ok_or_else(|| SocketError::InvalidAddress(addr.to_string()))?;
        self.socket
            .send_to(data, dest)
            .map_err(SocketError::SendFailed)
    }

    /// Receive data, returning (bytes_read, sender_address).
    pub fn receive(&self, buffer: &mut [u8]) -> Result<(usize, String), SocketError> {
        let (len, addr) = self
            .socket
            .recv_from(buffer)
            .map_err(SocketError::ReceiveFailed)?;
        Ok((len, addr.to_string()))
    }

    /// Set the socket to blocking or non-blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) {
        let _ = self.socket.set_nonblocking(nonblocking);
    }

    /// Set read and write timeouts on the socket.
    pub fn set_timeout(&self, duration: Duration) {
        let _ = self.socket.set_read_timeout(Some(duration));
        let _ = self.socket.set_write_timeout(Some(duration));
    }

    /// Returns the local address this socket is bound to.
    pub fn local_addr(&self) -> Option<String> {
        self.socket.local_addr().ok().map(|a| a.to_string())
    }
}

/// TCP listener wrapper.
pub struct TcpListener {
    listener: std::net::TcpListener,
}

impl TcpListener {
    /// Bind a TCP listener to the given address.
    pub fn bind(addr: &str) -> Result<Self, SocketError> {
        let listener = std::net::TcpListener::bind(addr).map_err(SocketError::BindFailed)?;
        Ok(Self { listener })
    }

    /// Accept an incoming connection, returning (TcpConnection, peer_address).
    pub fn accept(&self) -> Result<(TcpConnection, String), SocketError> {
        let (stream, addr) = self.listener.accept().map_err(SocketError::ReceiveFailed)?;
        Ok((TcpConnection { stream }, addr.to_string()))
    }

    /// Set the listener to blocking or non-blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) {
        let _ = self.listener.set_nonblocking(nonblocking);
    }

    /// Returns the local address this listener is bound to.
    pub fn local_addr(&self) -> Option<String> {
        self.listener.local_addr().ok().map(|a| a.to_string())
    }
}

/// TCP connection wrapper.
pub struct TcpConnection {
    stream: std::net::TcpStream,
}

impl TcpConnection {
    /// Connect to a remote TCP address.
    pub fn connect(addr: &str) -> Result<Self, SocketError> {
        let stream = std::net::TcpStream::connect(addr).map_err(SocketError::BindFailed)?;
        Ok(Self { stream })
    }

    /// Send data over the connection.
    pub fn send(&self, data: &[u8]) -> Result<usize, SocketError> {
        use std::io::Write;
        let mut stream = &self.stream;
        stream.write(data).map_err(SocketError::SendFailed)
    }

    /// Receive data into buffer, returning bytes_read.
    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize, SocketError> {
        use std::io::Read;
        let mut stream = &self.stream;
        let n = stream.read(buffer).map_err(SocketError::ReceiveFailed)?;
        if n == 0 {
            return Err(SocketError::ConnectionClosed);
        }
        Ok(n)
    }

    /// Get the peer address of this connection.
    pub fn peer_addr(&self) -> String {
        self.stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default()
    }

    /// Check if the connection appears to be active (has a valid peer address).
    pub fn is_connected(&self) -> bool {
        self.peer_addr() != ""
    }

    /// Set read and write timeouts on the connection.
    pub fn set_timeout(&self, duration: Duration) {
        let _ = self.stream.set_read_timeout(Some(duration));
        let _ = self.stream.set_write_timeout(Some(duration));
    }

    /// Set the connection to non-blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) {
        let _ = self.stream.set_nonblocking(nonblocking);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = NetworkConfig::default();
        assert_eq!(cfg.bind_address, "0.0.0.0");
        assert_eq!(cfg.port, 0);
        assert_eq!(cfg.protocol, Protocol::Udp);
        assert_eq!(cfg.max_packet_size, 65536);
        assert_eq!(cfg.timeout_ms, 5000);
    }

    #[test]
    fn test_udp_bind_send_receive() {
        let receiver = UdpSocket::bind("127.0.0.1:0").expect("bind receiver");
        let addr = receiver.local_addr().expect("local addr");

        let sender = UdpSocket::bind("127.0.0.1:0").expect("bind sender");
        let msg = b"hello udp";
        let sent = sender.send_to(msg, &addr).expect("send_to");
        assert_eq!(sent, msg.len());

        let mut buf = [0u8; 1024];
        let (n, from) = receiver.receive(&mut buf).expect("receive");
        assert_eq!(&buf[..n], msg);
        assert!(!from.is_empty());
    }

    #[test]
    fn test_tcp_listener_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");

        let client = TcpConnection::connect(&addr).expect("connect");
        let (server_conn, peer) = listener.accept().expect("accept");
        assert!(!peer.is_empty());

        let msg = b"hello tcp";
        let sent = client.send(msg).expect("send");
        assert_eq!(sent, msg.len());

        let mut buf = [0u8; 1024];
        let n = server_conn.receive(&mut buf).expect("receive");
        assert_eq!(&buf[..n], msg);
    }

    #[test]
    fn test_packet_queue() {
        let queue = PacketQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.push(NetworkPacket {
            data: vec![1, 2, 3],
            sender_addr: "127.0.0.1:1234".to_string(),
            timestamp: 100,
        });
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let pkt = queue.pop().expect("pop packet");
        assert_eq!(pkt.data, vec![1, 2, 3]);
        assert_eq!(pkt.sender_addr, "127.0.0.1:1234");
        assert_eq!(pkt.timestamp, 100);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_invalid_address_error() {
        let result = UdpSocket::bind("not_a_valid_address");
        assert!(result.is_err());
    }

    #[test]
    fn test_send_to_invalid_address() {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let result = socket.send_to(b"test", "no_such_host:9999");
        assert!(result.is_err());
    }
}
