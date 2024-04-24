use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_channel::mpsc::UnboundedSender;

pub type Tx = UnboundedSender<Message>;
pub type PeerMap = Arc<Mutex<Vec<Peer>>>;

pub struct Peer {
    pub user_id: String,
    pub addr: SocketAddr,
    pub tx: Tx,
}

impl Peer {
    pub fn new(user_id: String, addr: SocketAddr, tx: Tx) -> Peer {
        return Peer { user_id, addr, tx };
    }
}

pub trait PeerMethods {
    fn remove_peer(self: &Self, addr: SocketAddr) -> ();
    fn find_peer_id(self: &Self, addr: SocketAddr) -> String;
}

impl PeerMethods for PeerMap {
    fn remove_peer(self: &Self, addr: SocketAddr) -> () {
        let mut locked_peers = self.lock().unwrap();

        locked_peers.retain(|peer| peer.addr != addr);
    }

    fn find_peer_id(self: &Self, addr: SocketAddr) -> String {
        let locked_peers = self.lock().unwrap();
        let peer = locked_peers.iter().find(|&peer| peer.addr == addr).unwrap();

        return peer.user_id.clone();
    }
}
