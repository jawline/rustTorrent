use torrent::Info;
use reqwest;
use reqwest::header::ContentLength;
use bencoder;
use urlencode::urlencode;
use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::{thread, time};
use std::str;
use tracker::{TrackerState, PeerAddress};
use std::io::copy;

fn gen_tracker_request(info: &Info, peer_port: u16) -> String {
    let uploaded = 0;
    let downloaded = 0;
    let left = 0;
    let event = "started";

    format!("{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1",
        info.announce,
        urlencode(&info.info_hash),
        urlencode(&info.peer_id),
        peer_port,
        uploaded,
        downloaded,
        left,
        event)
}

pub fn http_tracker_do_announce(info: &Info, peer_port: u16) -> Result<bencoder::Entry, String> {
    let mut response = reqwest::get(&gen_tracker_request(info, peer_port)).expect("Failed to send request");
    if response.status() == reqwest::StatusCode::Ok {
        let len = response.headers().get::<ContentLength>()
            .map(|ct_len| **ct_len)
            .unwrap_or(0);
        let mut buf = Vec::with_capacity(len as usize);
        
        if let Err(v) = copy(&mut response, &mut buf) {
            Err(v.to_string())
        } else {
            let mut buf = &buf[0..];
            let info = bencoder::decode(&mut buf);

            match info {
                Ok(info) => Ok(info),
                Err(v) => Err("Bencoder decode error".to_string())
            }
        }
    } else {
        Err("Announce error".to_string())
    }
}

pub fn http_tracker_extract_peers(peers: &bencoder::Entry, send: &Sender<TrackerState>) {
    let mut extracted = Vec::new();

    if let bencoder::EntryData::Str(ref v) = &peers.data {
        //Binary strings model, 6 bytes representation, 4 bytes are IP 2 bytes are port all big endian

        for i in 0..v.len() / 6 {
            let mut data = &v[(i * 6) .. ((i * 6) + 6)];

            let ip = data.read_u32::<BE>().unwrap();
            let port = data.read_u16::<BE>().unwrap();

            extracted.push(PeerAddress {
                ip: IpAddr::V4(Ipv4Addr::from(ip)),
                port: port
            });
        }

    } else {
        println!("Dictionary Model");
        //Dictionary model, each entry has an ip and a port, ip might be IPv6 or IPv4
    }

    send.send(TrackerState::Announced(extracted));
}

pub fn http_tracker(info: &Info, peer_port: u16, tracker_port: u16, send: Sender<TrackerState>, recv: Receiver<TrackerState>) {

    loop {

        let announce_resp = http_tracker_do_announce(info, peer_port);

        if announce_resp.is_err() {
            send.send(TrackerState::Close("Announce resp error".to_string()));
            return;
        }

        let announce_resp = announce_resp.unwrap();

        let peers = announce_resp.field("peers");
        
        if let Ok(peers) = peers {
            http_tracker_extract_peers(&peers, &send);
        }

        let interval = announce_resp.field("interval");

        if interval.is_err() {
            send.send(TrackerState::Close("No interval error".to_string()));
            return;
        }

        let interval = interval.unwrap();

        thread::sleep(time::Duration::from_millis(interval.as_usize().unwrap() as u64));
    }
    
    send.send(TrackerState::Close("Broke HTTP Tracker".to_string()));
}

