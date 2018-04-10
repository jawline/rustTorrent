#![feature(try_trait)]
#[allow(dead_code)]

extern crate sha1;
extern crate url;
extern crate byteorder;
extern crate rand;
extern crate reqwest;

mod bencoder;
mod bencoder_recode;
mod torrent;
mod torrent_data;
mod tracker;
mod download;
mod peer_server;
mod peer_client;
mod peer_id;
mod urlencode;
mod bitfield;

use std::env;
use std::thread;
use std::time::Duration;

pub fn main() {
    let (master_send, master_recv) = download::download(&env::args().nth(1).unwrap());

    loop {
        let master_data = master_recv.try_recv();
        if let Ok(download::DownloadState::Close) = master_data {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }    
}
