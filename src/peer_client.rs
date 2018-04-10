/**
 * TCP Peer-wire client implementation
 */

use torrent::Info;
use tracker::PeerAddress;
use std::io;
use std::io::{copy, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time;
use std::sync::mpsc::{Sender, Receiver};
use byteorder::{BE, WriteBytesExt, ReadBytesExt};
use std::sync::mpsc;
use bitfield::Bitfield;

pub enum ClientState {
    Commit(usize, Vec<u8>), /* Write cached piece to file */
    Need(Bitfield),
    Want(usize),
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
            let mut payload = vec![0; (msg_len -1) as usize];

            //Does this message have a payload?
            if msg_len > 1 {
                stream.read_exact(&mut payload)?;
            }

            Ok(GeneralMsg {
                action: action,
                payload: payload
            })
        }
    }
}

fn request(stream: &mut TcpStream, piece: usize, start: usize, length: usize, piece_size: usize) -> Result<(), io::Error> {

    let length = if start + length > piece_size {
        piece_size - start
    } else {
        length
    };

    let mut request_data = Vec::with_capacity(12);
    request_data.write_u32::<BE>(piece as u32);
    request_data.write_u32::<BE>(start as u32);
    request_data.write_u32::<BE>(length as u32);

    let msg = GeneralMsg {
        action: 6,
        payload: request_data
    };

    stream.write(&msg.serialize())?;
    Ok(())
}

fn interested(stream: &mut TcpStream) -> Result<(), io::Error> {
    stream.write(&GeneralMsg{ action: 2, payload: Vec::new() }.serialize())?;
    Ok(())
}

fn unchoked(stream: &mut TcpStream) -> Result<(), io::Error> {
    stream.write(&GeneralMsg { action: 1, payload: Vec::new() }.serialize())?;
    Ok(())
}

const MAX_REQUEST_SIZE: usize = 16384;

pub fn peer_client(torrent: &Info, peer: &PeerAddress) -> (Sender<ClientState>, Receiver<ClientState>) {
    let torrent = torrent.clone();
    let peer = peer.clone();

    let (thread_send, main_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();

    let mut bitfield = Bitfield::new((0..torrent.pieces.len() / 8).map(|_| 0).collect());

    let mut am_choked = true;
    let mut am_interested = false;

    let mut am_needing = false;
    let mut am_acquiring = false;

    let mut acquiring = 0;
    let mut acquire_step = 0;
    let mut waiting_piece = false;
    let mut acquire_buffer = vec![0; torrent.piece_length];

    thread::spawn(move || {
        let mut client = TcpStream::connect((peer.ip, peer.port));

        if let Err(e) = client {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        let mut client = client.unwrap();

        client.set_read_timeout(Some(time::Duration::from_millis(500)));
        client.set_write_timeout(Some(time::Duration::from_millis(500)));
       
        let handshake = HandshakeMsg {
            pstr: "BitTorrent protocol".to_string(),
            info_hash: torrent.info_hash.clone(),
            peer_id: torrent.peer_id.clone()
        };

        if let Err(_) = client.write(&handshake.serialize()) {
            thread_send.send(ClientState::Close("Error sending BT peer-wire handshake".to_string()));
            return;
        }

        let handshake_recv = HandshakeMsg::recv(&mut client);
    
        if let Err(e) = handshake_recv {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        loop {

            if let Ok(msg) = thread_recv.try_recv() {
                match msg { 
                    ClientState::Want(piece) => {
                        acquiring = piece;
                        am_acquiring = true;
                        am_needing = false;
                        acquire_step = 0;
                    },
                    ClientState::Close(reason) => {
                        thread_send.send(ClientState::Close(reason));
                        return;
                    },
                    _ => {
                        thread_send.send(ClientState::Close("ctrl error".to_string()));
                        return;
                    }
                }
            }

            if !am_choked && am_acquiring && !waiting_piece {
                if acquire_step < torrent.piece_length {
                    request(&mut client, acquiring, acquire_step, MAX_REQUEST_SIZE, torrent.piece_length);
                    waiting_piece = true;
                } else {
                    thread_send.send(ClientState::Commit(acquiring, acquire_buffer.clone()));
                    am_acquiring = false;
                }
            }

            if !am_choked && !am_needing && !am_acquiring {
                thread_send.send(ClientState::Need(bitfield.clone()));
                am_needing = true;
            }

            let next = GeneralMsg::recv(&mut client);

            if let Ok(msg) = next { 
                match msg.action {
                    0 => /* Choke */ {
                        println!("Choked");
                        am_choked = true;
                        waiting_piece = false;
                    },
                    1 => /* Unchoked */ {
                        println!("Unchoked");
                        am_choked = false;
                    },
                    2 => /* Interested */ {
                        println!("Interested");
                        am_interested = true;
                    },
                    3 => /* Not Interested */ {
                        println!("Not Interested");
                        am_interested = false;
                    },
                    4 => /* Have */ {
                        let mut payload: &[u8] = &msg.payload;

                        if payload.len() == 4 {
                            let piece = payload.read_u32::<BE>().unwrap();
                            bitfield.set(piece as usize);
                        } else {
                            println!("Have - Bad payload");
                        }
                    },
                    5 => /* Bitfield */ {
                        bitfield = Bitfield::new(msg.payload);
                        interested(&mut client);
                        unchoked(&mut client);
                    },
                    7 => /* Piece */ {
                        let mut payload: &[u8] = &msg.payload;
                        let index = payload.read_u32::<BE>().unwrap();
                        let begin = payload.read_u32::<BE>().unwrap() as usize;
                        let length = payload.len();
                        let mut buffer_lock = &mut acquire_buffer[begin..begin + length];
                        copy(&mut payload, &mut buffer_lock).unwrap();
                        acquire_step = begin + length;
                        waiting_piece = false;
                    },
                    8 => {
                        println!("Cancel Received");
                    },
                    255 => /* Keep Alive */ {},
                    _ => {
                        thread_send.send(ClientState::Close(format!("Unhandled action {}", msg.action))).unwrap();
                        return;
                    }
                }
            } else if let Err(e) = next {
                if !(e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut) {
                    thread_send.send(ClientState::Close("we where EOF'd".to_string())).unwrap();
                    return;
                }
            }
        }
    });

    (main_send, main_recv)
}
