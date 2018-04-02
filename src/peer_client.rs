/**
 * TCP Peer-wire client implementation
 */

use torrent::Info;
use tracker::PeerAddress;
use std::io::Write;
use std::net::TcpStream;
use std::thread;

pub struct HandshakeMsg {
    pub pstr: String,
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>
}

impl HandshakeMsg {
    pub fn serialize(&self) -> Vec<u8> {
        let resvd = [0u8; 8];
        let mut data = Vec::new();
        data.write(&[self.pstr.len() as u8]).unwrap();
        data.write(self.pstr.as_bytes()).unwrap();
        data.write(&resvd).unwrap();
        data.write(&self.info_hash).unwrap();
        data.write(&self.peer_id).unwrap();
        data
    }
}

pub fn peer_client(torrent: &Info, peer: &PeerAddress) {
    let torrent = torrent.clone();
    let peer = peer.clone();
 
    thread::spawn(move || {
        let mut client = TcpStream::connect((peer.ip, peer.port)).unwrap();
        
        let handshake = HandshakeMsg {
            pstr: "BitTorrent protocol".to_string(),
            info_hash: torrent.info_hash.clone(),
            peer_id: torrent.peer_id.clone()
        };

        if let Err(_) = client.write(&handshake.serialize()) {
            println!("Error sending BT peer-wire handshake");
        }

        println!("Send BitTorrent wire handshake");
    });
}
