//! NAT traversal: STUN client, UDP hole punching, rendezvous server, and P2P connections.
//!
//! Enables peer-to-peer connections through NATs using STUN for address discovery
//! and UDP hole punching for direct connectivity.

use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket as StdUdpSocket};
use std::time::Duration;

use thiserror::Error;

/// Errors specific to NAT traversal.
#[derive(Debug, Error)]
pub enum NatError {
    #[error("STUN request failed: {0}")]
    StunFailed(String),
    #[error("STUN server unreachable: {0}")]
    StunUnreachable(String),
    #[error("hole punch failed: {0}")]
    HolePunchFailed(String),
    #[error("rendezvous failed: {0}")]
    RendezvousFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("timeout")]
    Timeout,
}

// ─── STUN Client ───────────────────────────────────────────────────────────────

/// Minimal STUN Binding Request/Response parser (RFC 5389 subset).
///
/// Only supports the Binding method with the MAPPED-ADDRESS attribute.
#[derive(Debug)]
pub struct StunClient {
    socket: StdUdpSocket,
    servers: Vec<String>,
    timeout: Duration,
}

/// Parsed STUN Binding Response.
#[derive(Debug, Clone)]
pub struct StunResponse {
    /// The public address as seen by the STUN server.
    pub mapped_addr: SocketAddr,
    /// The XOR-mapped address (if present).
    pub xor_mapped_addr: Option<SocketAddr>,
}

const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

impl StunClient {
    /// Create a new STUN client with default public STUN servers.
    pub fn new() -> Result<Self, NatError> {
        let socket = StdUdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        Ok(Self {
            socket,
            servers: vec![
                "stun.l.google.com:19302".to_string(),
                "stun1.l.google.com:19302".to_string(),
            ],
            timeout: Duration::from_secs(3),
        })
    }

    /// Create a STUN client with custom servers.
    pub fn with_servers(servers: Vec<String>) -> Result<Self, NatError> {
        let socket = StdUdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        Ok(Self {
            socket,
            servers,
            timeout: Duration::from_secs(3),
        })
    }

    /// Set the request timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        let _ = self.socket.set_read_timeout(Some(timeout));
    }

    /// Discover the public address of this client by sending STUN Binding Requests.
    pub fn discover_public_addr(&self) -> Result<SocketAddr, NatError> {
        for server in &self.servers {
            match self.query_server(server) {
                Ok(response) => return Ok(response.mapped_addr),
                Err(_) => continue,
            }
        }
        Err(NatError::StunFailed("all STUN servers failed".to_string()))
    }

    /// Query a specific STUN server and return the response.
    pub fn query_server(&self, server: &str) -> Result<StunResponse, NatError> {
        let addr = server
            .to_socket_addrs()
            .map_err(|e| NatError::InvalidAddress(e.to_string()))?
            .next()
            .ok_or_else(|| NatError::InvalidAddress(server.to_string()))?;

        let request = Self::build_binding_request();
        self.socket.send_to(&request, addr)?;

        let mut buf = [0u8; 1024];
        let (len, _from) = self
            .socket
            .recv_from(&mut buf)
            .map_err(|_| NatError::Timeout)?;

        Self::parse_binding_response(&buf[..len])
    }

    /// Build a STUN Binding Request packet.
    pub fn build_binding_request() -> Vec<u8> {
        let mut packet = Vec::with_capacity(20);
        packet.extend(&STUN_BINDING_REQUEST.to_be_bytes());
        packet.extend(&0u16.to_be_bytes());
        packet.extend(&STUN_MAGIC_COOKIE.to_be_bytes());
        let tx_id = Self::generate_transaction_id();
        packet.extend(&tx_id);
        packet
    }

    /// Parse a STUN Binding Response packet.
    pub fn parse_binding_response(data: &[u8]) -> Result<StunResponse, NatError> {
        if data.len() < 20 {
            return Err(NatError::StunFailed("response too short".to_string()));
        }

        let msg_type = u16::from_be_bytes([data[0], data[1]]);
        if msg_type != STUN_BINDING_RESPONSE {
            return Err(NatError::StunFailed(format!(
                "unexpected message type: 0x{:04x}",
                msg_type
            )));
        }

        let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        let mut offset = 20;
        let end = 20 + msg_len;

        let mut mapped_addr = None;
        let mut xor_mapped_addr = None;

        while offset + 4 <= end && offset + 4 <= data.len() {
            let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + attr_len > data.len() {
                break;
            }

            let attr_data = &data[offset..offset + attr_len];

            match attr_type {
                STUN_ATTR_MAPPED_ADDRESS => {
                    if let Some(addr) = Self::parse_mapped_address(attr_data) {
                        mapped_addr = Some(addr);
                    }
                }
                STUN_ATTR_XOR_MAPPED_ADDRESS => {
                    if let Some(addr) = Self::parse_xor_mapped_address(attr_data, &data[4..20]) {
                        xor_mapped_addr = Some(addr);
                    }
                }
                _ => {}
            }

            offset += (attr_len + 3) & !3;
        }

        let mapped = mapped_addr
            .or(xor_mapped_addr)
            .ok_or_else(|| NatError::StunFailed("no mapped address in response".to_string()))?;

        Ok(StunResponse {
            mapped_addr: mapped,
            xor_mapped_addr,
        })
    }

    fn parse_mapped_address(data: &[u8]) -> Option<SocketAddr> {
        if data.len() < 8 {
            return None;
        }
        let family = data[1];
        let port = u16::from_be_bytes([data[2], data[3]]);
        match family {
            0x01 => {
                if data.len() < 8 {
                    return None;
                }
                let ip = std::net::Ipv4Addr::new(data[4], data[5], data[6], data[7]);
                Some(SocketAddr::new(ip.into(), port))
            }
            0x02 => {
                if data.len() < 20 {
                    return None;
                }
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&data[4..20]);
                let ip = std::net::Ipv6Addr::from(octets);
                Some(SocketAddr::new(ip.into(), port))
            }
            _ => None,
        }
    }

    fn parse_xor_mapped_address(data: &[u8], tx_id: &[u8]) -> Option<SocketAddr> {
        if data.len() < 8 {
            return None;
        }
        let family = data[1];
        let port = u16::from_be_bytes([data[2], data[3]]) ^ (STUN_MAGIC_COOKIE >> 16) as u16;
        match family {
            0x01 => {
                if data.len() < 8 {
                    return None;
                }
                let cookie_bytes = STUN_MAGIC_COOKIE.to_be_bytes();
                let ip = std::net::Ipv4Addr::new(
                    data[4] ^ cookie_bytes[0],
                    data[5] ^ cookie_bytes[1],
                    data[6] ^ cookie_bytes[2],
                    data[7] ^ cookie_bytes[3],
                );
                Some(SocketAddr::new(ip.into(), port))
            }
            0x02 => {
                if data.len() < 20 || tx_id.len() < 12 {
                    return None;
                }
                let cookie_bytes = STUN_MAGIC_COOKIE.to_be_bytes();
                let mut octets = [0u8; 16];
                for i in 0..4 {
                    octets[i] = data[4 + i] ^ cookie_bytes[i];
                }
                for i in 0..12 {
                    octets[4 + i] = data[8 + i] ^ tx_id[i];
                }
                let ip = std::net::Ipv6Addr::from(octets);
                Some(SocketAddr::new(ip.into(), port))
            }
            _ => None,
        }
    }

    fn generate_transaction_id() -> [u8; 12] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut hasher = DefaultHasher::new();
        std::time::Instant::now()
            .elapsed()
            .as_nanos()
            .hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
        let hash = hasher.finish();
        let mut id = [0u8; 12];
        id[0..8].copy_from_slice(&hash.to_le_bytes());
        id[8..12].copy_from_slice(&(hash.wrapping_add(1)).to_le_bytes()[0..4]);
        id
    }
}

impl Default for StunClient {
    fn default() -> Self {
        Self::new().expect("failed to create STUN client")
    }
}

// ─── UDP Hole Punching ─────────────────────────────────────────────────────────

/// Coordinates UDP hole punching between two peers.
#[derive(Debug)]
pub struct HolePuncher {
    socket: StdUdpSocket,
}

impl HolePuncher {
    /// Create a new hole puncher with a bound UDP socket.
    pub fn bind(addr: &str) -> Result<Self, NatError> {
        let socket = StdUdpSocket::bind(addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { socket })
    }

    /// Perform UDP hole punching with a remote peer.
    ///
    /// Both peers should call this simultaneously. Sends packets to the
    /// remote address to create NAT mapping entries, then waits for a
    /// response to confirm connectivity.
    pub fn punch_hole(&self, remote_addr: &str) -> Result<(), NatError> {
        let addr = remote_addr
            .to_socket_addrs()
            .map_err(|e| NatError::InvalidAddress(e.to_string()))?
            .next()
            .ok_or_else(|| NatError::InvalidAddress(remote_addr.to_string()))?;

        let punch_data = b"PUNCH";
        for _ in 0..5 {
            let _ = self.socket.send_to(punch_data, addr);
            std::thread::sleep(Duration::from_millis(50));
        }

        let mut buf = [0u8; 1024];
        match self.socket.recv_from(&mut buf) {
            Ok(_) => Ok(()),
            Err(e) => Err(NatError::HolePunchFailed(e.to_string())),
        }
    }

    /// Punch hole and return the socket for direct communication.
    pub fn punch_hole_and_get_socket(self, remote_addr: &str) -> Result<StdUdpSocket, NatError> {
        self.punch_hole(remote_addr)?;
        Ok(self.socket)
    }

    /// Get the local address this socket is bound to.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.local_addr().ok()
    }
}

// ─── Rendezvous Server ─────────────────────────────────────────────────────────

/// A lightweight rendezvous server that brokers peer introductions.
#[derive(Debug)]
pub struct RendezvousServer {
    clients: HashMap<u64, SocketAddr>,
    pending_pairs: HashMap<u64, u64>,
    results: Vec<RendezvousResult>,
}

/// Result of a rendezvous pairing.
#[derive(Debug, Clone)]
pub struct RendezvousResult {
    pub peer_a: u64,
    pub peer_b: u64,
    pub addr_a: SocketAddr,
    pub addr_b: SocketAddr,
}

impl RendezvousServer {
    /// Create a new rendezvous server.
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            pending_pairs: HashMap::new(),
            results: Vec::new(),
        }
    }

    /// Register a client with their public address.
    pub fn register_client(&mut self, client_id: u64, addr: SocketAddr) {
        self.clients.insert(client_id, addr);
    }

    /// Request pairing between two clients. Returns result if both are registered.
    pub fn pair_clients(&mut self, peer_a: u64, peer_b: u64) -> Option<RendezvousResult> {
        let addr_a = self.clients.get(&peer_a)?;
        let addr_b = self.clients.get(&peer_b)?;

        let result = RendezvousResult {
            peer_a,
            peer_b,
            addr_a: *addr_a,
            addr_b: *addr_b,
        };
        self.results.push(result.clone());
        self.clients.remove(&peer_a);
        self.clients.remove(&peer_b);
        self.pending_pairs.remove(&peer_a);
        self.pending_pairs.remove(&peer_b);
        Some(result)
    }

    /// Queue a pairing request. The pair completes when both clients register.
    pub fn request_pair(&mut self, client_id: u64, peer_id: u64) {
        self.pending_pairs.insert(client_id, peer_id);
    }

    /// Check if pending pairs can now be completed.
    pub fn try_complete_pairs(&mut self) -> Vec<RendezvousResult> {
        let pairs: Vec<(u64, u64)> = self
            .pending_pairs
            .iter()
            .filter(|(a, _)| self.clients.contains_key(a))
            .filter_map(|(a, b)| {
                if self.clients.contains_key(b) {
                    Some((*a, *b))
                } else {
                    None
                }
            })
            .collect();

        let mut results = Vec::new();
        for (a, b) in pairs {
            if let Some(result) = self.pair_clients(a, b) {
                results.push(result);
            }
        }
        results
    }

    /// Get the number of registered clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get completed results and clear them.
    pub fn drain_results(&mut self) -> Vec<RendezvousResult> {
        std::mem::take(&mut self.results)
    }

    /// Check if a client is registered.
    pub fn is_registered(&self, client_id: u64) -> bool {
        self.clients.contains_key(&client_id)
    }

    /// Remove a client from the registry.
    pub fn unregister_client(&mut self, client_id: u64) {
        self.clients.remove(&client_id);
    }
}

impl Default for RendezvousServer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── P2P Connection ────────────────────────────────────────────────────────────

/// A peer-to-peer connection wrapping a hole-punched UDP socket.
#[derive(Debug)]
pub struct P2pConnection {
    socket: StdUdpSocket,
    peer_addr: SocketAddr,
    connected: bool,
}

impl P2pConnection {
    /// Create a P2P connection from an existing socket and peer address.
    pub fn new(socket: StdUdpSocket, peer_addr: SocketAddr) -> Self {
        Self {
            socket,
            peer_addr,
            connected: true,
        }
    }

    /// Perform hole punching and create a P2P connection.
    pub fn connect(local_addr: &str, remote_addr: &str) -> Result<Self, NatError> {
        let puncher = HolePuncher::bind(local_addr)?;
        puncher.punch_hole(remote_addr)?;

        let peer_addr = remote_addr
            .to_socket_addrs()
            .map_err(|e| NatError::InvalidAddress(e.to_string()))?
            .next()
            .ok_or_else(|| NatError::InvalidAddress(remote_addr.to_string()))?;

        let socket = puncher.punch_hole_and_get_socket(remote_addr)?;

        Ok(Self {
            socket,
            peer_addr,
            connected: true,
        })
    }

    /// Send data to the peer.
    pub fn send(&self, data: &[u8]) -> Result<usize, NatError> {
        if !self.connected {
            return Err(NatError::HolePunchFailed("not connected".to_string()));
        }
        Ok(self.socket.send_to(data, self.peer_addr)?)
    }

    /// Receive data from the peer.
    pub fn receive(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), NatError> {
        if !self.connected {
            return Err(NatError::HolePunchFailed("not connected".to_string()));
        }
        let (len, addr) = self.socket.recv_from(buffer)?;
        Ok((len, addr))
    }

    /// Check if the connection is active.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the remote peer's address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Get the local address.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.local_addr().ok()
    }

    /// Close the connection.
    pub fn close(&mut self) {
        self.connected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_build_binding_request() {
        let request = StunClient::build_binding_request();
        assert_eq!(request.len(), 20);
        assert_eq!(request[0], 0x00);
        assert_eq!(request[1], 0x01);
        assert_eq!(request[4], 0x21);
        assert_eq!(request[5], 0x12);
        assert_eq!(request[6], 0xA4);
        assert_eq!(request[7], 0x42);
    }

    #[test]
    fn test_stun_parse_binding_response_too_short() {
        let result = StunClient::parse_binding_response(&[0, 0, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_stun_parse_binding_response_wrong_type() {
        let mut data = vec![0u8; 20];
        data[0] = 0x01;
        data[1] = 0x11;
        let result = StunClient::parse_binding_response(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stun_parse_mapped_address_ipv4() {
        let data = [0x00, 0x01, 0x1F, 0x90, 0xC0, 0xA8, 0x01, 0x01];
        let result = StunClient::parse_mapped_address(&data);
        assert!(result.is_some());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 8080);
        assert_eq!(
            addr.ip(),
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1))
        );
    }

    #[test]
    fn test_stun_parse_mapped_address_too_short() {
        let data = [0x00, 0x01, 0x1F];
        assert!(StunClient::parse_mapped_address(&data).is_none());
    }

    #[test]
    fn test_stun_parse_mapped_address_unknown_family() {
        let data = [0x00, 0x03, 0x1F, 0x90, 0xC0, 0xA8, 0x01, 0x01];
        assert!(StunClient::parse_mapped_address(&data).is_none());
    }

    #[test]
    fn test_rendezvous_server_register_and_pair() {
        let mut server = RendezvousServer::new();
        let addr_a: SocketAddr = "1.2.3.4:1000".parse().unwrap();
        let addr_b: SocketAddr = "5.6.7.8:2000".parse().unwrap();

        server.register_client(1, addr_a);
        server.register_client(2, addr_b);
        assert_eq!(server.client_count(), 2);

        let result = server.pair_clients(1, 2);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.peer_a, 1);
        assert_eq!(r.peer_b, 2);
        assert_eq!(r.addr_a, addr_a);
        assert_eq!(r.addr_b, addr_b);
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_rendezvous_server_pair_one_missing() {
        let mut server = RendezvousServer::new();
        server.register_client(1, "1.2.3.4:1000".parse().unwrap());
        let result = server.pair_clients(1, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_rendezvous_server_request_and_complete() {
        let mut server = RendezvousServer::new();
        server.request_pair(1, 2);

        server.register_client(1, "1.2.3.4:1000".parse().unwrap());
        assert!(server.try_complete_pairs().is_empty());

        server.register_client(2, "5.6.7.8:2000".parse().unwrap());
        let results = server.try_complete_pairs();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].peer_a, 1);
    }

    #[test]
    fn test_rendezvous_server_unregister() {
        let mut server = RendezvousServer::new();
        server.register_client(1, "1.2.3.4:1000".parse().unwrap());
        assert!(server.is_registered(1));
        server.unregister_client(1);
        assert!(!server.is_registered(1));
    }

    #[test]
    fn test_rendezvous_server_drain_results() {
        let mut server = RendezvousServer::new();
        server.register_client(1, "1.2.3.4:1000".parse().unwrap());
        server.register_client(2, "5.6.7.8:2000".parse().unwrap());
        server.pair_clients(1, 2);

        let results = server.drain_results();
        assert_eq!(results.len(), 1);
        assert!(server.drain_results().is_empty());
    }

    #[test]
    fn test_hole_puncher_bind() {
        let puncher = HolePuncher::bind("127.0.0.1:0");
        assert!(puncher.is_ok());
        let p = puncher.unwrap();
        assert!(p.local_addr().is_some());
    }

    #[test]
    fn test_stun_client_with_servers() {
        let client = StunClient::with_servers(vec!["127.0.0.1:3478".to_string()]);
        assert!(client.is_ok());
    }

    #[test]
    fn test_stun_generate_transaction_id() {
        let id1 = StunClient::generate_transaction_id();
        let id2 = StunClient::generate_transaction_id();
        assert_eq!(id1.len(), 12);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_rendezvous_result_fields() {
        let mut server = RendezvousServer::new();
        server.register_client(10, "10.0.0.1:5000".parse().unwrap());
        server.register_client(20, "10.0.0.2:6000".parse().unwrap());
        let r = server.pair_clients(10, 20).unwrap();
        assert_eq!(r.addr_a.port(), 5000);
        assert_eq!(r.addr_b.port(), 6000);
    }
}
