/**
 * TCP Peer-wire client implementation
 */

use torrent::Info;
use tracker::PeerAddress;
use std::io;
use std::io::{copy, Read, Write};
use std::io::ErrorKind::{WouldBlock, TimedOut};
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

struct PeerClient { 

    send: Sender<ClientState>,
    recv: Receiver<ClientState>,

    stream: TcpStream,

    piece_length: usize,

    bitfield: Bitfield,

    am_choked: bool,
    am_interested: bool,

    am_needing: bool,

    am_acquiring: bool,
    acquiring_piece: usize,
    acquire_step: usize,
    waiting_piece: bool,
    acquire_buffer: Vec<u8>
}

impl PeerClient {

    pub fn sync_ctrl(&mut self) -> bool {
        if let Ok(msg) = self.recv.try_recv() {
            match msg { 
                ClientState::Want(piece) => {
                    self.acquiring_piece = piece;
                    self.am_acquiring = true;
                    self.am_needing = false;
                    self.acquire_step = 0;
                    true
                },
                ClientState::Close(reason) => {
                    self.send.send(ClientState::Close(reason));
                    false
                },
                _ => {
                    self.send.send(ClientState::Close("ctrl error".to_string()));
                    false
                }
            }
        } else {
            true
        }
    }

    pub fn update_state(&mut self) {
        if !self.am_choked && self.am_acquiring && !self.waiting_piece {
            if self.acquire_step < self.piece_length {
                request(&mut self.stream, self.acquiring_piece, self.acquire_step, MAX_REQUEST_SIZE, self.piece_length);
                self.waiting_piece = true;
            } else {
                self.send.send(ClientState::Commit(self.acquiring_piece, self.acquire_buffer.clone()));
                self.am_acquiring = false;
            }
        }

        if !self.am_choked && !self.am_needing && !self.am_acquiring {
            self.send.send(ClientState::Need(self.bitfield.clone()));
            self.am_needing = true;
        }
    }

    pub fn process_msg(&mut self, msg: GeneralMsg) -> bool {
        match msg.action {
            0 => /* Choke */ {
                println!("Choked");
                self.am_choked = true;
                self.waiting_piece = false;
            },
            1 => /* Unchoked */ {
                println!("Unchoked");
                self.am_choked = false;
            },
            2 => /* Interested */ {
                println!("Interested");
                self.am_interested = true;
            },
            3 => /* Not Interested */ {
                println!("Not Interested");
                self.am_interested = false;
            },
            4 => /* Have */ {
                let mut payload: &[u8] = &msg.payload;

                if payload.len() == 4 {
                    let piece = payload.read_u32::<BE>().unwrap();
                    self.bitfield.set(piece as usize);
                } else {
                    println!("Have - Bad payload");
                }
            },
            5 => /* Bitfield */ {
                self.bitfield = Bitfield::new(msg.payload);
                interested(&mut self.stream);
                unchoked(&mut self.stream);
            },
            7 => /* Piece */ {
                let mut payload: &[u8] = &msg.payload;
                let index = payload.read_u32::<BE>().unwrap();
                if index as usize == self.acquiring_piece {
                    let begin = payload.read_u32::<BE>().unwrap() as usize;
                    let length = payload.len();
                    let mut buffer_lock = &mut self.acquire_buffer[begin..begin + length];
                    copy(&mut payload, &mut buffer_lock).unwrap();
                    self.acquire_step = begin + length;
                    self.waiting_piece = false;
                }
            },
            8 => {
                println!("Cancel Received");
            },
            255 => /* Keep Alive */ {},
            _ => {
                self.send.send(ClientState::Close(format!("Unhandled action {}", msg.action))).unwrap();
                return false
            }
        };
        
        true
    }

}

pub fn peer_client(torrent: &Info, peer: &PeerAddress) -> (Sender<ClientState>, Receiver<ClientState>) {
    let torrent = torrent.clone();
    let peer = peer.clone();

    let (thread_send, main_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<ClientState>, Receiver<ClientState>) = mpsc::channel();

    thread::spawn(move || {
        let client = TcpStream::connect((peer.ip, peer.port));

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

        let handshake_recv = HandshakeMsg::recv(&mut client);
    
        if let Err(e) = handshake_recv {
            thread_send.send(ClientState::Close(e.to_string()));
            return;
        }

        

        client.set_read_timeout(Some(time::Duration::from_millis(500)));
        client.set_write_timeout(Some(time::Duration::from_millis(500)));

        let mut client = PeerClient {
            send: thread_send,
            recv: thread_recv,

            stream: client,
            piece_length: torrent.piece_length,

            bitfield: Bitfield::new((0..torrent.pieces.len() / 8).map(|_| 0).collect()),

            am_choked: true,
            am_interested: false,

            am_acquiring: false,
            am_needing: false,

            acquiring_piece: 0,
            acquire_step: 0,
            waiting_piece: false,
            acquire_buffer: vec![0; torrent.piece_length]
        };

        loop {

            if !client.sync_ctrl() {
                return;
            }

            client.update_state();            

            let next = GeneralMsg::recv(&mut client.stream);

            if let Ok(msg) = next {
                 client.process_msg(msg);
            } else if let Err(e) = next {
                match e.kind() {
                    WouldBlock | TimedOut => {},
                    _ => {
                        client.send.send(ClientState::Close(e.to_string())).unwrap();
                        return;
                    }
                };
            }
        }
    });

    (main_send, main_recv)
}
