use std::net::IpAddr;

pub enum TrackerState {
    Connected(u64),
    Announced(Vec<PeerAddress>),
    Close(String)
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PeerAddress {
    pub ip: IpAddr,
    pub port: u16
}
