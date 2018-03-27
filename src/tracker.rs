use torrent::Info;
use url::{Url};
use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use std::net::UdpSocket;
use std::io::Error;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

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
 * Main / thread sync
 */

pub enum TrackerData {
    Connected,
    Close
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
    pub action: u32,
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
        res.write_u32::<BE>(self.action).unwrap();
        res.write_u32::<BE>(self.transaction_id).unwrap();
        
        //TODO: Info and peer ID strings

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
pub struct ConnectResp {
    pub connection_id: u64
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

const CONNECT_RESP_SIZE: usize = 16;

/**
 * Tracker Logic
 */

fn udp_do_connect(url: &Url, socket: &mut UdpSocket) -> Result<ConnectResp, MsgError> {
    let connect = ConnectCmd { action: 0, transaction_id: 23131 };
    socket.send_to(&connect.serialize(), url).expect("couldn't send data");
    let mut resp = [0; CONNECT_RESP_SIZE];
    if let Ok(v) = socket.recv(&mut resp) {
        ConnectResp::deserialize(23131, &resp)
    } else {
        Err("Could not receive".to_string())
    }
}

fn udp_do_announce(url: &Url, socket: &mut UdpSocket) -> Result<(), MsgError> {

    let connect = AnnounceCmd {
        action: 0,
        transaction_id: 23131
    };

    socket.send_to(&connect.serialize(), url).expect("couldn't send data");
    let mut resp = [0; CONNECT_RESP_SIZE];
    Ok(()) 
}

pub fn tracker_thread(info: &Info, sender: Sender<TrackerData>, recv: Receiver<TrackerData>) {
    if info.announce.starts_with("udp://") {

        let announce = Url::parse(&info.announce.to_string());
        let udp_addr = "0.0.0.0:19696";

        let announce = announce.unwrap();

        println!("UDP Tracker {} to {}", udp_addr, announce);

        let mut socket = UdpSocket::bind(udp_addr).expect("couldn't bind to address");

        let connection = udp_do_connect(&announce, &mut socket);

        if connection.is_err() {
            sender.send(TrackerData::Close);
            return;
        }

        let connection = connection.unwrap().connection_id;

        println!("Got Connection ID {}", connection);
    }
}

pub fn connect(info: &Info) -> (Sender<TrackerData>, Receiver<TrackerData>) {
    let info = info.clone();

    let (thread_send, main_recv): (Sender<TrackerData>, Receiver<TrackerData>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<TrackerData>, Receiver<TrackerData>) = mpsc::channel();

    thread::spawn(move || {
        tracker_thread(&info, thread_send, thread_recv);
    });
   
    (main_send, main_recv)
}