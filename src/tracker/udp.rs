use torrent::Info;
use std::io::Write;
use url::Url;
use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::mpsc::{Sender, Receiver};
use std::{thread, time};
use tracker::{TrackerState, PeerAddress};

/**
 * Error Handlers
 */

type MsgError = String;

fn cerr<T: Sized, S: Sized + ToString>(r: Result<T, S>) -> Result<T, MsgError> {
    match r {
        Ok(v) => Ok(v),
        Err(r) => Err(r.to_string())
    }
}

/**
 * Command serialization
 */

trait Command {
    fn serialize(&self) -> Vec<u8>;
}

pub struct ConnectCmd {
    pub action: u32,
    pub transaction_id: u32
}

pub struct AnnounceCmd {
    pub connection_id: u64,
    pub transaction_id: u32,
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
    pub downloaded: u64,
    pub left: u64,
    pub uploaded: u64,
    pub event: u32,
    pub ip: u32,
    pub key: u32,
    pub num_want: u32,
    pub port: u16
}

impl Command for ConnectCmd {
    fn serialize(&self) -> Vec<u8> {
        let mut res = vec![];
        res.write_u64::<BE>(0x41727101980).unwrap(); //Connect magic
        res.write_u32::<BE>(self.action).unwrap();
        res.write_u32::<BE>(self.transaction_id).unwrap();
        res 
    }
}

impl Command for AnnounceCmd {
    fn serialize(&self) -> Vec<u8> {
        let mut res = vec![];
        res.write_u64::<BE>(self.connection_id).unwrap();
        res.write_u32::<BE>(1).unwrap();
        res.write_u32::<BE>(self.transaction_id).unwrap();

        res.write(&self.info_hash).unwrap();
        res.write(&self.peer_id).unwrap();

        res.write_u64::<BE>(self.downloaded).unwrap();
        res.write_u64::<BE>(self.left).unwrap();
        res.write_u64::<BE>(self.uploaded).unwrap();
        res.write_u32::<BE>(self.event).unwrap();
        res.write_u32::<BE>(self.ip).unwrap();
        res.write_u32::<BE>(self.key).unwrap();
        res.write_u32::<BE>(self.num_want).unwrap();
        res.write_u16::<BE>(self.port).unwrap();
        res
    }
}

/**
 * Response Deserialization
 */

trait Response {
    fn deserialize(transaction_id: u32, data: &[u8]) -> Result<Self, MsgError> where Self: Sized;
}

#[derive(Debug)]
#[derive(Clone)]
pub struct ConnectResp {
    pub connection_id: u64
}

#[derive(Debug)]
pub struct AnnounceResp {
    pub interval: u32,
    pub leechers: u32,
    pub seeders: u32,
    pub peers: Vec<PeerAddress>
}

impl Response for ConnectResp {
    fn deserialize(transaction_id: u32, mut data: &[u8]) -> Result<ConnectResp, MsgError> {
        let action = cerr(data.read_u32::<BE>())?;
        let tranid = cerr(data.read_u32::<BE>())?;

        if tranid != transaction_id {
            return Err("Bad transaction ID".to_string());
        }

        if action != 0 {
            return Err("Bad action ID".to_string());
        }

        let conid = cerr(data.read_u64::<BE>())?;
        
        Ok(ConnectResp {
            connection_id: conid
        })
    }
}

impl Response for AnnounceResp {
    fn deserialize(transaction_id: u32, mut data: &[u8]) -> Result<AnnounceResp, MsgError> {
        let action = cerr(data.read_u32::<BE>())?;
        let tran_id = cerr(data.read_u32::<BE>())?;

        if action != 1 {
            return Err("Announce error (Bad Action)".to_string());
        }

        if tran_id != transaction_id {
            return Err("Transaction ID error".to_string());
        }

        let interval = cerr(data.read_u32::<BE>())?;
        let leechers = cerr(data.read_u32::<BE>())?;
        let seeders = cerr(data.read_u32::<BE>())?;

        let num_peers = data.len() / IP_SIZE;
        let mut peers = Vec::new();

        for _ in 0..num_peers {
            let ip = cerr(data.read_u32::<BE>())?;
            let port = cerr(data.read_u16::<BE>())?;
            peers.push(PeerAddress {
                ip: IpAddr::V4(Ipv4Addr::from(ip)),
                port: port
            });
        }

        Ok(AnnounceResp {
            interval: interval,
            leechers: leechers,
            seeders: seeders,
            peers: peers
        }) 
    }
}

const CONNECT_RESP_SIZE: usize = 16;
const ANNOUNCE_RESP_SIZE: usize = 20;
const IP_SIZE: usize = 6;
const NUM_WANT: usize = 50;

/**
 * Tracker Logic
 */

fn udp_do_connect(url: &Url, socket: &mut UdpSocket) -> Result<ConnectResp, MsgError> {
    let connect = ConnectCmd { action: 0, transaction_id: 23131 };
    socket.send_to(&connect.serialize(), url).expect("couldn't send data");
    let mut resp = [0; CONNECT_RESP_SIZE];
    if let Ok(v) = socket.recv(&mut resp) {
        ConnectResp::deserialize(23131, &resp[0..v])
    } else {
        Err("Could not receive".to_string())
    }
}

fn udp_do_announce(url: &Url, connection: u64, peer_port: u16, info_hash: &[u8], peer_id: &[u8], socket: &mut UdpSocket) -> Result<AnnounceResp, MsgError> {

    let announce = AnnounceCmd {
        connection_id: connection, 
        transaction_id: 23131,
        info_hash: info_hash.to_vec(),
        peer_id: peer_id.to_vec(),
        downloaded: 0,
        left: 0,
        uploaded: 0,
        event: 0,
        ip: 0,
        key: 0,
        num_want: NUM_WANT as u32,
        port: peer_port as u16
    };

    socket.send_to(&announce.serialize(), url).expect("couldn't send data");
    let mut resp = [0; ANNOUNCE_RESP_SIZE + IP_SIZE + (IP_SIZE * NUM_WANT)];
    
    if let Ok(v) = socket.recv(&mut resp) {
        Ok(AnnounceResp::deserialize(23131, &resp[0..v])?)
    } else {
        Err("Could not receive announce resp".to_string())
    }
}

pub fn udp_tracker(info: &Info, peer_port: u16, tracker_port: u16, sender: Sender<TrackerState>, recv: Receiver<TrackerState>) {
    let announce = Url::parse(&info.announce.to_string());
    let udp_addr = "0.0.0.0:".to_string() + &tracker_port.to_string();

    let announce = announce.unwrap();

    println!("UDP Tracker {} to {}", udp_addr, announce);

    let mut socket = UdpSocket::bind(udp_addr).expect("couldn't bind to address");

    let connection = udp_do_connect(&announce, &mut socket);

    if let Err(v) = connection {
        sender.send(TrackerState::Close(v.clone())).unwrap();
        return;
    }

    let connection = connection.unwrap().connection_id;
    sender.send(TrackerState::Connected(connection));

    loop {
        let announced = udp_do_announce(&announce, connection, peer_port, &info.info_hash, &info.peer_id, &mut socket); 

        if let Err(v) = announced {
            sender.send(TrackerState::Close(v.clone())).unwrap();
            return;
        }

        let announced = announced.unwrap();

        sender.send(TrackerState::Announced(announced.peers.clone()));
        
        thread::sleep(time::Duration::from_millis(announced.interval as u64));
    }
}
