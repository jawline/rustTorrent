/**
 * TCP Peer-wire client implementation
 */

use torrent::Info;
use tracker::PeerAddress;
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time;
use std::sync::mpsc::{Sender, Receiver};
use byteorder::{BE, WriteBytesExt, ReadBytesExt};
use std::sync::mpsc;
use bitfield::Bitfield;

pub enum ClientState {
    Close(String)
}

#[derive(Debug)]
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

    pub fn recv(stream: &mut TcpStream) -> Result<HandshakeMsg, io::Error> {
        let pstrlen = stream.read_u8()?;

        let mut pstr = vec![0; pstrlen as usize];
        stream.read_exact(&mut pstr)?;

        let mut resvd = vec![0; 8];
        stream.read_exact(&mut resvd)?;

        let mut info_hash = vec![0; 20];
        let mut peer_id = vec![0; 20];

        stream.read_exact(&mut info_hash)?;
        stream.read_exact(&mut peer_id)?;
        
        Ok(HandshakeMsg {
            pstr: String::from_utf8(pstr).unwrap() /* TODO: This might error */,
            info_hash: info_hash,
            peer_id: peer_id
        })
    }
}

#[derive(Debug)]
pub struct GeneralMsg {
    pub action: u8,
    pub payload: Vec<u8>
}

impl GeneralMsg {
    
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.write_u32::<BE>((self.payload.len() + 1) as u32).unwrap();
        data.write(&[self.action]).unwrap();
        data.write(&self.payload).unwrap();
        data
    }

    pub fn recv(stream: &mut TcpStream) -> Result<GeneralMsg, io::Error> {
        let msg_len = stream.read_u32::<BE>()?;

        if msg_len == 0 {
            //Keep alive message we call action 255 internally (TODO: This might break something?)
            Ok(GeneralMsg {
                action: 255,
                payload: Vec::new()
            })
        } else {
            let action = stream.read_u8()?;
            let mut payload = vec![0; (msg_len - 1) as usize];
            stream.read(&mut payload)?;
            Ok(GeneralMsg {
                action: action,
                payload: payload
            })
        }
    }
}

pub fn peer_client(torrent: &Info, peer: &PeerAddress) -> (Sender<ClientState>, Receiver<ClientState>) {
    let torrent = torrent.clone();
    let peer = peer.clone();

    let (thread_send, main_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();

    let mut bitfield = Bitfield::new((0..torrent.pieces.len() / 8).map(|_| 0).collect());
    let mut am_choked = 1;
    let mut am_interested = 0;

    thread::spawn(move || {
        let mut client = TcpStream::connect((peer.ip, peer.port));

        if let Err(e) = client {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        let mut client = client.unwrap();

        client.set_read_timeout(Some(time::Duration::from_millis(5000)));
        client.set_write_timeout(Some(time::Duration::from_millis(5000)));
       
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

        let handshake_recv = HandshakeMsg::recv(&mut client);
    
        if let Err(e) = handshake_recv {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        println!("Handshake Received");

        loop {
            let next = GeneralMsg::recv(&mut client);

            if let Err(e) = next {
                thread_send.send(ClientState::Close(e.to_string()));
                return;
            }

            let msg = next.unwrap();

            match msg.action {
                0 => /* Choke */ {
                    println!("Choked");
                    am_choked = 1;
                },
                1 => /* Unchoked */ {
                    println!("Unchoked");
                    am_choked = 0;
                },
                2 => /* Interested */ {
                    println!("Interested");
                    am_interested = 1;
                },
                3 => /* Not Interested */ {
                    println!("Not Interested");
                    am_interested = 0;
                },
                4 => /* Have */ {
                    let mut payload: &[u8] = &msg.payload;

                    if payload.len() == 4 {
                        let piece = payload.read_u32::<BE>().unwrap();
                        bitfield.set(piece as usize, true);
                        println!("Have {}", piece);
                    } else {
                        println!("Have - Bad payload");
                    }
                },
                5 => /* Bitfield */ {
                    bitfield = Bitfield::new(msg.payload);
                    println!("Bitfield set");
                },
                255 => {
                    println!("Keep-Alive");
                },
                _ => {
                    thread_send.send(ClientState::Close(format!("Unhandled action {}", msg.action))).unwrap();
                    return;
                }
            }

        }
    });

    (main_send, main_recv)
}
