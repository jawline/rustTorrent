/**
 * TCP Peer-wire client implementation
 */

use torrent::Info;
use tracker::PeerAddress;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use byteorder::{BE, WriteBytesExt, ReadBytesExt};
use std::sync::mpsc;

pub enum ClientState {
    Close(String)
}

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

pub struct GeneralMsg {
    pub action: u8,
    pub payload: Vec<u8>
}

impl GeneralMsg {
    
    pub fn serialize(&self) -> Vec<u8> {
        let resvd = [0u8; 8];
        let mut data = Vec::new();
        data.write_u32::<BE>(self.payload.len() as u32).unwrap();
        data.write(&[self.action]).unwrap();
        data.write(&self.payload);
        data
    }

    pub fn recv(stream: &mut TcpStream) -> Result<GeneralMsg, io::Error> {
    }
}

pub fn peer_client(torrent: &Info, peer: &PeerAddress) -> (Sender<ClientState>, Receiver<ClientState>) {
    let torrent = torrent.clone();
    let peer = peer.clone();

    let (thread_send, main_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();

    thread::spawn(move || {
        let mut client = TcpStream::connect((peer.ip, peer.port));

        if let Err(e) = client {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        let mut client = client.unwrap();
       
        let handshake = HandshakeMsg {
            pstr: "BitTorrent protocol".to_string(),
            info_hash: torrent.info_hash.clone(),
            peer_id: torrent.peer_id.clone()
        };

        if let Err(_) = client.write(&handshake.serialize()) {
            thread_send.send(ClientState::Close("Error sending BT peer-wire handshake".to_string()));
            return;
        }

        println!("Send BitTorrent wire handshake");
        thread_send.send(ClientState::Close("Finished".to_string()));
    });

    (main_send, main_recv)
}
